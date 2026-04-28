use crate::desktop::graph::DesktopGraph;
use crate::input::FocusNode;
use crate::surface::SurfaceNode;

pub fn enforce_binding(graph: &mut DesktopGraph, focus: FocusNode, surface: SurfaceNode) {
    graph.focus = Some(focus);
    graph.active_surface = Some(surface);
}
