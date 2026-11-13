//! Split views ("docking"): every document, or a view of a document, lives in a
//! pane of an `egui_tiles` tree. Tabs can be dragged onto the edges of any pane
//! to split it horizontally or vertically.

use egui_tiles::{Container, LinearDir, Tile, TileId, Tiles, Tree};
use op_core::i18n::tr;
use serde::{Deserialize, Serialize};

/// What a pane shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum View {
    /// The full editor of the document.
    Editor,
    /// Petri net: Pre / Post / incidence matrices.
    Matrices,
    /// Petri net: properties, invariants, soundness.
    Analysis,
    /// Petri net: table of reachable markings.
    States,
    /// Petri net: reachability / coverability graph.
    Graph,
}

impl View {
    pub fn label(self) -> &'static str {
        match self {
            View::Editor => "",
            View::Matrices => tr("Matrices"),
            View::Analysis => tr("Analysis"),
            View::States => tr("States"),
            View::Graph => tr("Reachability graph"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Pane {
    pub doc: u64,
    pub view: View,
}

pub fn new_tree() -> Tree<Pane> {
    Tree::new_tabs(egui::Id::unique("open_process_dock"), Vec::new())
}

/// All tiles holding a pane of document `doc`.
pub fn panes_of(tree: &Tree<Pane>, doc: u64) -> Vec<TileId> {
    tree.tiles
        .iter()
        .filter_map(|(id, t)| match t {
            Tile::Pane(p) if p.doc == doc => Some(*id),
            _ => None,
        })
        .collect()
}

pub fn find(tree: &Tree<Pane>, pane: Pane) -> Option<TileId> {
    tree.tiles.find_pane(&pane)
}

/// Removes every pane of a document.
pub fn remove_doc(tree: &mut Tree<Pane>, doc: u64) {
    for id in panes_of(tree, doc) {
        remove_tile(tree, id);
    }
}

pub fn remove_tile(tree: &mut Tree<Pane>, id: TileId) {
    if let Some(parent) = tree.tiles.parent_of(id)
        && let Some(Tile::Container(c)) = tree.tiles.get_mut(parent)
    {
        c.remove_child(id);
    }
    tree.tiles.remove(id);
    if tree.root == Some(id) {
        tree.root = None;
    }
}

/// Tabs container holding `tile` (if its parent is a tab bar).
fn tabs_parent(tiles: &Tiles<Pane>, tile: TileId) -> Option<TileId> {
    let parent = tiles.parent_of(tile)?;
    matches!(tiles.get(parent), Some(Tile::Container(Container::Tabs(_)))).then_some(parent)
}

/// First tabs container of the tree (depth-first).
fn first_tabs(tree: &Tree<Pane>) -> Option<TileId> {
    let mut stack = vec![tree.root?];
    while let Some(id) = stack.pop() {
        match tree.tiles.get(id) {
            Some(Tile::Container(Container::Tabs(_))) => return Some(id),
            Some(Tile::Container(c)) => stack.extend(c.children_vec().into_iter().rev()),
            _ => {}
        }
    }
    None
}

/// Adds a pane as a new tab next to `near` (or in the first tab bar) and activates it.
pub fn add_tab(tree: &mut Tree<Pane>, pane: Pane, near: Option<TileId>) -> TileId {
    let tile = tree.tiles.insert_pane(pane);
    let target = near.and_then(|n| tabs_parent(&tree.tiles, n)).or_else(|| first_tabs(tree));
    match target {
        Some(tabs_id) => {
            if let Some(Tile::Container(Container::Tabs(tabs))) = tree.tiles.get_mut(tabs_id) {
                tabs.add_child(tile);
                tabs.set_active(tile);
            }
        }
        None => match tree.root {
            None => {
                tree.root = Some(tree.tiles.insert_tab_tile(vec![tile]));
            }
            Some(root) => {
                let tabs = tree.tiles.insert_tab_tile(vec![tile]);
                tree.root = Some(tree.tiles.insert_horizontal_tile(vec![root, tabs]));
            }
        },
    }
    tile
}

/// Opens `pane` in a new tab bar placed right of (or below) the tab bar of `near`.
pub fn add_beside(tree: &mut Tree<Pane>, pane: Pane, near: Option<TileId>, vertical: bool) -> TileId {
    let tile = tree.tiles.insert_pane(pane);
    let new_tabs = tree.tiles.insert_tab_tile(vec![tile]);
    let dir = if vertical { LinearDir::Vertical } else { LinearDir::Horizontal };
    let anchor = near.and_then(|n| tabs_parent(&tree.tiles, n).or(Some(n))).or(tree.root);
    let Some(anchor) = anchor else {
        tree.root = Some(new_tabs);
        return tile;
    };
    let parent = tree.tiles.parent_of(anchor);
    // Same-direction linear parent: insert right after the anchor.
    if let Some(p) = parent
        && let Some(Tile::Container(Container::Linear(lin))) = tree.tiles.get_mut(p)
        && lin.dir == dir
    {
        let idx = lin
            .children
            .iter()
            .position(|c| *c == anchor)
            .map(|i| i + 1)
            .unwrap_or(lin.children.len());
        lin.children.insert(idx, new_tabs);
        return tile;
    }
    // Otherwise wrap the anchor and the new tab bar in a new linear container.
    let wrapper = if vertical {
        tree.tiles.insert_vertical_tile(vec![anchor, new_tabs])
    } else {
        tree.tiles.insert_horizontal_tile(vec![anchor, new_tabs])
    };
    match parent {
        None => tree.root = Some(wrapper),
        Some(p) => {
            if let Some(Tile::Container(c)) = tree.tiles.get_mut(p) {
                match c {
                    Container::Tabs(t) => {
                        if let Some(i) = t.children.iter().position(|x| *x == anchor) {
                            t.children[i] = wrapper;
                        }
                        if t.active == Some(anchor) {
                            t.active = Some(wrapper);
                        }
                    }
                    Container::Linear(l) => {
                        if let Some(i) = l.children.iter().position(|x| *x == anchor) {
                            l.children[i] = wrapper;
                        }
                    }
                    Container::Grid(_) => {
                        // Rare: fall back to wrapping the whole layout.
                        let root = tree.root.unwrap_or(anchor);
                        let w = tree.tiles.insert_horizontal_tile(vec![root, new_tabs]);
                        tree.root = Some(w);
                    }
                }
            }
        }
    }
    tile
}

/// Makes the tab holding `tile` visible.
pub fn activate(tree: &mut Tree<Pane>, tile: TileId) {
    tree.make_active(|id, _| id == tile);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tabs_and_split() {
        let mut t = new_tree();
        let a = add_tab(
            &mut t,
            Pane {
                doc: 1,
                view: View::Editor,
            },
            None,
        );
        let _b = add_tab(
            &mut t,
            Pane {
                doc: 2,
                view: View::Editor,
            },
            Some(a),
        );
        let c = add_beside(
            &mut t,
            Pane {
                doc: 2,
                view: View::Matrices,
            },
            Some(a),
            false,
        );
        let root = t.root.unwrap();
        match t.tiles.get(root) {
            Some(Tile::Container(Container::Linear(l))) => assert_eq!(l.children.len(), 2),
            other => panic!("expected a horizontal split, got {other:?}"),
        }
        assert_eq!(panes_of(&t, 2).len(), 2);
        remove_doc(&mut t, 2);
        assert!(panes_of(&t, 2).is_empty());
        assert!(t.tiles.get(c).is_none());
    }
}
