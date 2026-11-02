use crate::modules::{IconKind, ModuleEvent, ToolDef};
pub const ICON: IconKind = IconKind::Svg(include_bytes!("../../../assets/icons/v2/vpJoin.svg"));
pub fn tool() -> ToolDef {
    ToolDef {
        id: "VPJOIN",
        label: "Join",
        icon: ICON,
        event: ModuleEvent::Command("VPJOIN".to_string()),
    }
}
