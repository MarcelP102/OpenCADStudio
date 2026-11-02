use crate::modules::{IconKind, ModuleEvent, ToolDef};
pub const ICON: IconKind = IconKind::Svg(include_bytes!("../../../assets/icons/v2/viewports.svg"));
pub fn tool() -> ToolDef {
    ToolDef {
        id: "VPORTS_NAMED",
        label: "Named",
        icon: ICON,
        event: ModuleEvent::Command("VPORTS".to_string()),
    }
}
