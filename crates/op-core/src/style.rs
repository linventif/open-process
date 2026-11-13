//! Display style of a diagram (saved with the model).

use crate::i18n::tr;
use serde::{Deserialize, Serialize};

/// How links (arcs, sequence flows) are drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EdgeStyle {
    /// The usual style of the notation: orthogonal for BPMN, straight for Petri nets.
    #[default]
    Auto,
    Straight,
    /// Right angles (90°).
    Orthogonal,
    Curved,
}

impl EdgeStyle {
    pub const ALL: [EdgeStyle; 4] = [EdgeStyle::Auto, EdgeStyle::Straight, EdgeStyle::Orthogonal, EdgeStyle::Curved];

    pub fn label(self) -> &'static str {
        match self {
            EdgeStyle::Auto => tr("style.automatic"),
            EdgeStyle::Straight => tr("style.straight"),
            EdgeStyle::Orthogonal => tr("style.right_angles_90"),
            EdgeStyle::Curved => tr("style.curved"),
        }
    }

    /// Resolves `Auto` with the default of the notation.
    pub fn or(self, default: EdgeStyle) -> EdgeStyle {
        if self == EdgeStyle::Auto { default } else { self }
    }
}

/// How Petri net transitions are drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransitionShape {
    /// Rectangle with the name inside (notation used in class).
    #[default]
    Box,
    /// Thin vertical bar with the name below (common in tools like PIPE/TINA).
    Bar,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DiagramStyle {
    #[serde(default)]
    pub edges: EdgeStyle,
    /// Hand-drawn ("sketch") rendering.
    #[serde(default)]
    pub sketch: bool,
    /// Petri nets only.
    #[serde(default)]
    pub transitions: TransitionShape,
}
