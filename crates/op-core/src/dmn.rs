//! Editable DMN decision tables and an explicitly bounded unary-test evaluator.
//! This is not a general FEEL interpreter or a complete DMN conformance level.
use crate::i18n::{key, tr, trf};
use crate::xml::{self, XmlWriter};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataType {
    String,
    Number,
    Boolean,
}
impl DataType {
    pub const ALL: [Self; 3] = [Self::String, Self::Number, Self::Boolean];
    pub fn type_ref(self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Number => "number",
            Self::Boolean => "boolean",
        }
    }
    fn accepts(self, v: &Value) -> bool {
        v.is_null()
            || match self {
                Self::String => v.is_string(),
                Self::Number => v.is_number(),
                Self::Boolean => v.is_boolean(),
            }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HitPolicy {
    Unique,
    First,
    Any,
    Collect,
}
impl HitPolicy {
    pub const ALL: [Self; 4] = [Self::Unique, Self::First, Self::Any, Self::Collect];
    pub fn code(self) -> &'static str {
        match self {
            Self::Unique => "UNIQUE",
            Self::First => "FIRST",
            Self::Any => "ANY",
            Self::Collect => "COLLECT",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Clause {
    pub name: String,
    pub data_type: DataType,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rule {
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub annotation: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DecisionTable {
    pub name: String,
    pub hit_policy: HitPolicy,
    pub inputs: Vec<Clause>,
    pub outputs: Vec<Clause>,
    pub rules: Vec<Rule>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Evaluation {
    pub matched_rules: Vec<usize>,
    pub results: Vec<BTreeMap<String, Value>>,
}

impl DecisionTable {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            hit_policy: HitPolicy::Unique,
            inputs: vec![Clause {
                name: "input".into(),
                data_type: DataType::Number,
            }],
            outputs: vec![Clause {
                name: "result".into(),
                data_type: DataType::String,
            }],
            rules: vec![],
        }
    }
    pub fn add_input(&mut self) {
        if self.inputs.len() >= 128 {
            return;
        }
        let name = unique_name("input", self.inputs.iter().map(|c| c.name.as_str()));
        self.inputs.push(Clause {
            name,
            data_type: DataType::Number,
        });
        for r in &mut self.rules {
            r.inputs.push("-".into());
        }
    }
    pub fn add_output(&mut self) {
        if self.outputs.len() >= 128 {
            return;
        }
        let name = unique_name("result", self.outputs.iter().map(|c| c.name.as_str()));
        self.outputs.push(Clause {
            name,
            data_type: DataType::String,
        });
        for r in &mut self.rules {
            r.outputs.push("\"\"".into());
        }
    }
    pub fn add_rule(&mut self) {
        if self.rules.len() >= 10_000 {
            return;
        }
        self.rules.push(Rule {
            inputs: vec!["-".into(); self.inputs.len()],
            outputs: self
                .outputs
                .iter()
                .map(|c| {
                    match c.data_type {
                        DataType::String => "\"\"",
                        DataType::Number => "0",
                        DataType::Boolean => "false",
                    }
                    .into()
                })
                .collect(),
            annotation: String::new(),
        });
    }
    pub fn remove_input(&mut self, index: usize) {
        if self.inputs.len() > 1 && index < self.inputs.len() {
            self.inputs.remove(index);
            for r in &mut self.rules {
                r.inputs.remove(index);
            }
        }
    }
    pub fn remove_output(&mut self, index: usize) {
        if self.outputs.len() > 1 && index < self.outputs.len() {
            self.outputs.remove(index);
            for r in &mut self.rules {
                r.outputs.remove(index);
            }
        }
    }
    pub fn check_storage(&self) -> Result<(), String> {
        if self.inputs.is_empty()
            || self.outputs.is_empty()
            || self.inputs.len() > 128
            || self.outputs.len() > 128
            || self.rules.len() > 10_000
        {
            return Err(tr("dmn.invalid_decision_table_dimensions").into());
        }
        if self
            .rules
            .iter()
            .any(|r| r.inputs.len() != self.inputs.len() || r.outputs.len() != self.outputs.len())
        {
            return Err(tr("dmn.rule_cells_do_not_match_decision_columns").into());
        }
        Ok(())
    }
    pub fn validate(&self) -> Vec<String> {
        if let Err(e) = self.check_storage() {
            return vec![e];
        }
        let mut issues = vec![];
        for clauses in [&self.inputs, &self.outputs] {
            let mut names = BTreeSet::new();
            if clauses.iter().any(|c| c.name.trim().is_empty() || !names.insert(&c.name)) {
                issues.push(tr("dmn.decision_columns_must_have_unique_non_empty").into());
            }
        }
        for (i, r) in self.rules.iter().enumerate() {
            for cell in &r.inputs {
                if let Err(e) = unary_test(cell, &Value::Null, 0) {
                    issues.push(trf("dmn.rule_error", &[&(i + 1), &e]));
                }
            }
            for (c, cell) in self.outputs.iter().zip(&r.outputs) {
                match literal(cell) {
                    Ok(v) if c.data_type.accepts(&v) => {}
                    _ => issues.push(trf("dmn.rule_invalid_output", &[&(i + 1), &c.name])),
                }
            }
        }
        issues
    }
    pub fn evaluate(&self, values: &BTreeMap<String, Value>) -> Result<Evaluation, String> {
        let issues = self.validate();
        if !issues.is_empty() {
            return Err(issues.join("\n"));
        }
        for c in &self.inputs {
            if !values.get(&c.name).is_some_and(|v| c.data_type.accepts(v)) {
                return Err(trf("dmn.missing_invalid_input", &[&c.name]));
            }
        }
        let mut matched_rules = vec![];
        let mut results = vec![];
        for (i, r) in self.rules.iter().enumerate() {
            let mut matches = true;
            for (c, cell) in self.inputs.iter().zip(&r.inputs) {
                matches &= unary_test(cell, &values[&c.name], 0)?;
            }
            if !matches {
                continue;
            }
            matched_rules.push(i + 1);
            let row = self
                .outputs
                .iter()
                .zip(&r.outputs)
                .map(|(c, s)| Ok((c.name.clone(), literal(s)?)))
                .collect::<Result<BTreeMap<_, _>, String>>()?;
            results.push(row);
            if self.hit_policy == HitPolicy::First {
                break;
            }
        }
        if self.hit_policy == HitPolicy::Unique && results.len() > 1 {
            return Err(tr("dmn.unique_policy_matched_several_rules").into());
        }
        if self.hit_policy == HitPolicy::Any {
            if results.windows(2).any(|p| p[0].iter().any(|(k, v)| !value_equal(v, &p[1][k]))) {
                return Err(tr("dmn.any_policy_produced_different_outputs").into());
            }
            results.truncate(1);
        }
        Ok(Evaluation { matched_rules, results })
    }
    pub fn to_csv(&self) -> String {
        let quote = |s: &str| format!("\"{}\"", s.replace('"', "\"\""));
        let mut lines = vec![
            self.inputs
                .iter()
                .map(|c| quote(&format!("input:{}", c.name)))
                .chain(self.outputs.iter().map(|c| quote(&format!("output:{}", c.name))))
                .chain(std::iter::once(quote("annotation")))
                .collect::<Vec<_>>()
                .join(","),
        ];
        for r in &self.rules {
            lines.push(
                r.inputs
                    .iter()
                    .chain(&r.outputs)
                    .chain(std::iter::once(&r.annotation))
                    .map(|s| quote(s))
                    .collect::<Vec<_>>()
                    .join(","),
            );
        }
        lines.join("\r\n")
    }
    pub fn to_xml(&self) -> String {
        let mut w = XmlWriter::new();
        w.open(
            "definitions",
            &[
                ("xmlns", "https://www.omg.org/spec/DMN/20230324/MODEL/".into()),
                ("id", "definitions_1".into()),
                ("name", self.name.clone()),
                ("namespace", "urn:open-process:dmn".into()),
            ],
        );
        w.open("decision", &[("id", "decision_1".into()), ("name", self.name.clone())]);
        w.open(
            "decisionTable",
            &[("id", "table_1".into()), ("hitPolicy", self.hit_policy.code().into())],
        );
        for (i, c) in self.inputs.iter().enumerate() {
            w.open("input", &[("id", format!("input_{i}")), ("label", c.name.clone())]);
            w.open(
                "inputExpression",
                &[("id", format!("expression_{i}")), ("typeRef", c.data_type.type_ref().into())],
            );
            w.text_element("text", &[], &c.name);
            w.close("inputExpression");
            w.close("input");
        }
        for (i, c) in self.outputs.iter().enumerate() {
            w.empty(
                "output",
                &[
                    ("id", format!("output_{i}")),
                    ("name", c.name.clone()),
                    ("typeRef", c.data_type.type_ref().into()),
                ],
            );
        }
        w.empty("annotation", &[("name", "Annotation".into())]);
        for (i, r) in self.rules.iter().enumerate() {
            w.open("rule", &[("id", format!("rule_{i}"))]);
            for (j, s) in r.inputs.iter().enumerate() {
                w.open("inputEntry", &[("id", format!("input_entry_{i}_{j}"))]);
                w.text_element("text", &[], s);
                w.close("inputEntry");
            }
            for (j, s) in r.outputs.iter().enumerate() {
                w.open("outputEntry", &[("id", format!("output_entry_{i}_{j}"))]);
                w.text_element("text", &[], s);
                w.close("outputEntry");
            }
            w.open("annotationEntry", &[]);
            w.text_element("text", &[], &r.annotation);
            w.close("annotationEntry");
            w.close("rule");
        }
        w.close("decisionTable");
        w.close("decision");
        w.close("definitions");
        w.out
    }
}
fn unique_name<'a>(prefix: &str, names: impl Iterator<Item = &'a str>) -> String {
    let names: BTreeSet<_> = names.collect();
    (1..).map(|i| format!("{prefix}{i}")).find(|n| !names.contains(n.as_str())).unwrap()
}
fn literal(src: &str) -> Result<Value, String> {
    let v = serde_json::from_str::<Value>(src.trim()).map_err(|_| trf("dmn.unsupported_decision_expression", &[&src]))?;
    if v.is_object() || v.is_array() {
        return Err(trf("dmn.unsupported_decision_expression", &[&src]));
    }
    Ok(v)
}
fn value_equal(a: &Value, b: &Value) -> bool {
    match (a.as_f64(), b.as_f64()) {
        (Some(a), Some(b)) => a == b,
        _ => a == b,
    }
}
fn unary_test(src: &str, value: &Value, depth: usize) -> Result<bool, String> {
    let src = src.trim();
    let unsupported = || trf("dmn.unsupported_decision_expression", &[&src]);
    if depth > 32 || src.len() > 4096 {
        return Err(unsupported());
    }
    if src == "-" {
        return Ok(true);
    }
    if let Some(inner) = src.strip_prefix("not(").and_then(|s| s.strip_suffix(')')) {
        return Ok(!unary_test(inner, value, depth + 1)?);
    }
    let mut quoted = false;
    let mut escape = false;
    let mut nesting = 0i32;
    let mut start = 0;
    let mut parts = vec![];
    for (i, c) in src.char_indices() {
        if quoted {
            if escape {
                escape = false;
            } else if c == '\\' {
                escape = true;
            } else if c == '"' {
                quoted = false;
            }
            continue;
        }
        match c {
            '"' => quoted = true,
            '[' | '(' => nesting += 1,
            ']' | ')' => nesting -= 1,
            ',' if nesting == 0 => {
                parts.push(&src[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    if !parts.is_empty() {
        parts.push(&src[start..]);
        let mut matched = false;
        for p in parts {
            matched |= unary_test(p, value, depth + 1)?;
        }
        return Ok(matched);
    }
    if src.contains("..") && src.len() >= 5 && !src.starts_with('"') {
        let (left, right) = (src.chars().next().unwrap(), src.chars().last().unwrap());
        if !matches!(left, '[' | '(' | ']') || !matches!(right, ']' | ')' | '[') {
            return Err(unsupported());
        }
        let (a, b) = src[1..src.len() - 1].split_once("..").ok_or_else(unsupported)?;
        let a = literal(a)?.as_f64().ok_or_else(unsupported)?;
        let b = literal(b)?.as_f64().ok_or_else(unsupported)?;
        if a > b {
            return Err(unsupported());
        }
        return Ok(value
            .as_f64()
            .is_some_and(|n| (if left == '[' { n >= a } else { n > a }) && (if right == ']' { n <= b } else { n < b })));
    }
    for op in ["<=", ">=", "<", ">", "="] {
        if let Some(rhs) = src.strip_prefix(op) {
            let rhs = literal(rhs)?;
            if op == "=" {
                return Ok(value_equal(value, &rhs));
            }
            let n = rhs.as_f64().ok_or_else(unsupported)?;
            return Ok(value.as_f64().is_some_and(|v| match op {
                "<=" => v <= n,
                ">=" => v >= n,
                "<" => v < n,
                _ => v > n,
            }));
        }
    }
    Ok(value_equal(value, &literal(src)?))
}

pub fn accepts_xml(src: &str) -> bool {
    xml::root_name(src).as_deref() == Some("definitions")
        && xml::parse(src).is_ok_and(|root| root.children.iter().any(|c| c.name == "decision"))
}
pub fn from_xml(src: &str) -> Result<DecisionTable, String> {
    let root = xml::parse(src)?;
    let unsupported = || tr("dmn.only_single_decision_table_supported_clauses_can").to_owned();
    if root.name != "definitions" || root.children.len() != 1 || root.children[0].name != "decision" {
        return Err(unsupported());
    }
    let decision = &root.children[0];
    if decision.children.len() != 1 || decision.children[0].name != "decisionTable" {
        return Err(unsupported());
    }
    let table = &decision.children[0];
    if table.attr("aggregation").is_some() {
        return Err(unsupported());
    }
    let hit_policy = match table.attr("hitPolicy").unwrap_or("UNIQUE") {
        "UNIQUE" => HitPolicy::Unique,
        "FIRST" => HitPolicy::First,
        "ANY" => HitPolicy::Any,
        "COLLECT" => HitPolicy::Collect,
        _ => return Err(unsupported()),
    };
    let data_type = |s: &str| match s {
        "string" => Ok(DataType::String),
        "number" => Ok(DataType::Number),
        "boolean" => Ok(DataType::Boolean),
        _ => Err(unsupported()),
    };
    let mut d = DecisionTable {
        name: decision.attr("name").unwrap_or("Decision").into(),
        hit_policy,
        inputs: vec![],
        outputs: vec![],
        rules: vec![],
    };
    for child in &table.children {
        match child.name.as_str() {
            "input" => {
                if child.children.len() != 1 || child.children[0].name != "inputExpression" {
                    return Err(unsupported());
                }
                let expression = &child.children[0];
                if expression.children.len() != 1 || expression.children[0].name != "text" {
                    return Err(unsupported());
                }
                d.inputs.push(Clause {
                    name: expression.children[0].text.clone(),
                    data_type: data_type(expression.attr("typeRef").unwrap_or("string"))?,
                });
            }
            "output" => {
                if !child.children.is_empty() {
                    return Err(unsupported());
                }
                d.outputs.push(Clause {
                    name: child.attr("name").unwrap_or("").into(),
                    data_type: data_type(child.attr("typeRef").unwrap_or("string"))?,
                });
            }
            "annotation" => {
                if !child.children.is_empty() {
                    return Err(unsupported());
                }
            }
            "rule" => {
                let mut r = Rule {
                    inputs: vec![],
                    outputs: vec![],
                    annotation: String::new(),
                };
                let mut annotations = 0;
                for entry in &child.children {
                    if entry.children.len() != 1 || entry.children[0].name != "text" {
                        return Err(unsupported());
                    }
                    let s = entry.children[0].text.clone();
                    match entry.name.as_str() {
                        "inputEntry" => r.inputs.push(s),
                        "outputEntry" => r.outputs.push(s),
                        "annotationEntry" => {
                            r.annotation = s;
                            annotations += 1;
                            if annotations > 1 {
                                return Err(unsupported());
                            }
                        }
                        _ => return Err(unsupported()),
                    }
                }
                d.rules.push(r);
            }
            _ => return Err(unsupported()),
        }
    }
    d.check_storage()?;
    Ok(d)
}
pub fn example() -> DecisionTable {
    let mut d = DecisionTable::new(tr(key("dmn.discount_decision")));
    d.inputs[0].name = "amount".into();
    d.outputs[0] = Clause {
        name: "discount".into(),
        data_type: DataType::Number,
    };
    d.rules = vec![
        Rule {
            inputs: vec!["< 100".into()],
            outputs: vec!["0".into()],
            annotation: String::new(),
        },
        Rule {
            inputs: vec![">= 100".into()],
            outputs: vec!["10".into()],
            annotation: String::new(),
        },
    ];
    d
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn policies_and_boundaries_are_executable() {
        let mut d = example();
        let input = |n| BTreeMap::from([("amount".into(), Value::from(n))]);
        assert_eq!(d.evaluate(&input(99)).unwrap().results[0]["discount"], 0);
        assert_eq!(d.evaluate(&input(100)).unwrap().results[0]["discount"], 10);
        d.rules[0].inputs[0] = "-".into();
        assert!(d.evaluate(&input(100)).is_err());
        d.hit_policy = HitPolicy::First;
        assert_eq!(d.evaluate(&input(100)).unwrap().matched_rules, [1]);
        d.hit_policy = HitPolicy::Any;
        assert!(d.evaluate(&input(100)).is_err());
        d.rules[1].outputs[0] = "0".into();
        assert_eq!(d.evaluate(&input(100)).unwrap().results.len(), 1);
        d.hit_policy = HitPolicy::Collect;
        assert_eq!(d.evaluate(&input(100)).unwrap().results.len(), 2);
    }
    #[test]
    fn unary_tests_support_ranges_strings_lists_and_reject_unknown_expressions() {
        assert!(unary_test("[1..10)", &Value::from(1), 0).unwrap());
        assert!(!unary_test("[1..10)", &Value::from(10), 0).unwrap());
        assert!(unary_test("\"a,b\", \"c\"", &Value::from("a,b"), 0).unwrap());
        assert!(unary_test("not(true)", &Value::from(false), 0).unwrap());
        assert!(unary_test("customer.age > 18", &Value::from(20), 0).is_err());
        assert!(unary_test("[9..1]", &Value::from(5), 0).is_err());
    }
    #[test]
    fn xml_and_json_roundtrip_and_foreign_models_are_not_silently_truncated() {
        let mut d = example();
        d.name = "Discount & approval".into();
        d.rules[0].annotation = "Notes < 100".into();
        assert_eq!(from_xml(&d.to_xml()).unwrap(), d);
        assert_eq!(
            serde_json::from_str::<DecisionTable>(&serde_json::to_string(&d).unwrap()).unwrap(),
            d
        );
        assert!(from_xml(&d.to_xml().replace("</definitions>", "<decision id=\"other\"/></definitions>")).is_err());
        assert!(from_xml(&d.to_xml().replace("<decisionTable ", "<decisionTable aggregation=\"SUM\" ")).is_err());
        d.add_input();
        d.add_output();
        d.add_rule();
        assert!(d.check_storage().is_ok());
        d.remove_input(0);
        d.remove_output(0);
        assert!(d.check_storage().is_ok());
    }
}
