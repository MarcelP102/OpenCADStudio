use crate::modules::{IconKind, ModuleEvent, ToolDef};
pub const ICON: IconKind = IconKind::Svg(include_bytes!("../../../assets/icons/v2/tileV.svg"));
pub fn tool() -> ToolDef {
    ToolDef {
        id: "VERTICAL",
        label: "Tile\nVert.",
        icon: ICON,
        event: ModuleEvent::Command("VERTICAL".to_string()),
    }
}
