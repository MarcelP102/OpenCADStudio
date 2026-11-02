use crate::modules::{IconKind, ModuleEvent, ToolDef};
pub const ICON: IconKind = IconKind::Svg(include_bytes!("../../../assets/icons/v2/zIn.svg"));
pub fn tool() -> ToolDef {
    ToolDef {
        id: "ZOOM_IN",
        label: "Zoom In",
        icon: ICON,
        event: ModuleEvent::Command("ZOOM IN".to_string()),
    }
}
