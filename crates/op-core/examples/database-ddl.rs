//! Produce the actual generator output for independent SQLite engine checks.
use op_core::database::*;
use op_core::geom::Pos;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::path::PathBuf::from(std::env::args_os().nth(1).ok_or("output directory required")?);
    std::fs::create_dir_all(&dir)?;
    let mut cases = vec![("shop", example())];
    let mut composite = Schema::new("Composite keys and cycles");
    let a = composite.add_table(Pos::default());
    let b = composite.add_table(Pos::new(400.0, 0.0));
    composite.tables[0].name = "parent".into();
    composite.tables[1].name = "child".into();
    for table in &mut composite.tables {
        table.columns[0].auto_increment = false;
    }
    let tenant = composite.add_column(a).unwrap();
    let c = composite.tables[0].columns.iter_mut().find(|c| c.id == tenant).unwrap();
    c.name = "tenant".into();
    c.data_type = SqlType::Integer;
    c.primary_key = true;
    c.nullable = false;
    composite.add_foreign_key(b, a).unwrap();
    composite.add_foreign_key(a, b).unwrap();
    cases.push(("composite-cycle", composite));
    let mut defaults = Schema::new("Literal defaults");
    let t = defaults.add_table(Pos::default());
    defaults.tables[0].name = "select \" a`b".into();
    let text = defaults.add_column(t).unwrap();
    let c = defaults.tables[0].columns.iter_mut().find(|c| c.id == text).unwrap();
    c.name = "quoted".into();
    c.data_type = SqlType::Text;
    c.default = DefaultValue::Text("O'Brien\\path'); DROP TABLE anything;--".into());
    let number = defaults.add_column(t).unwrap();
    let c = defaults.tables[0].columns.iter_mut().find(|c| c.id == number).unwrap();
    c.name = "quantity".into();
    c.data_type = SqlType::Integer;
    c.default = DefaultValue::Number("7".into());
    let boolean = defaults.add_column(t).unwrap();
    let c = defaults.tables[0].columns.iter_mut().find(|c| c.id == boolean).unwrap();
    c.name = "active".into();
    c.data_type = SqlType::Boolean;
    c.default = DefaultValue::Boolean(true);
    cases.push(("literal-defaults", defaults));
    for (name, schema) in cases {
        std::fs::write(dir.join(format!("{name}.json")), serde_json::to_vec_pretty(&schema)?)?;
        for (dialect, label) in [
            (Dialect::PostgreSql, "postgresql"),
            (Dialect::MySql, "mysql"),
            (Dialect::Sqlite, "sqlite"),
        ] {
            let sql = generate(&schema, dialect).map_err(|issues| format!("{name}: {issues:?}"))?;
            std::fs::write(dir.join(format!("{name}.{label}.sql")), sql)?;
        }
    }
    println!("DDL fixtures written to {}", dir.display());
    Ok(())
}
