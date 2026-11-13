//! # op-core
//!
//! GUI-independent core of **Open Process**: BPMN 2.0 and Petri net models,
//! analysis algorithms and file formats. It compiles to native and WebAssembly.

pub mod activity;
pub mod analysis;
pub mod answers;
pub mod bpmn;
pub mod bpmn_xml;
pub mod convert;
pub mod database;
pub mod dmn;
pub mod document;
pub mod epc;
pub mod evaluation;
pub mod examples;
pub mod flow;
pub mod geom;
pub mod i18n;
pub mod mining;
pub mod petri;
pub mod pnml;
pub mod style;
pub mod uml;
pub mod whiteboard;
pub mod xml;
