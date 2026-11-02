//! The line-icon set: the icons drawn on the 19-unit frame with 1-pixel
//! lines that `ui::icons` snaps to the pixel grid (see `ui::icon_snap`).
//!
//! Most tools take their icon straight from their `ToolDef` (an asset file
//! that now holds the new drawing, or one in `assets/icons/v2`). This module
//! holds the icons of command lines that have no ribbon tool of their own —
//! menu-bar rows such as `ZOOM P` or `VSCURRENT H`, and commands whose
//! registry entry used a chrome glyph — so the menu bar, the Quick Access
//! Toolbar and the command search show the same icon set.

/// Continue (`acont`).
pub static ACONT: &[u8] = include_bytes!("../../assets/icons/arc/arc_cont.svg");
/// Angular (`angular`).
pub static ANGULAR: &[u8] = include_bytes!("../../assets/icons/v2/angular.svg");
/// Load application (`app`).
pub static APP: &[u8] = include_bytes!("../../assets/icons/v2/app.svg");
/// Attribute Display (`attdisp`).
pub static ATTDISP: &[u8] = include_bytes!("../../assets/icons/v2/attdisp.svg");
/// Edit (`bedit`).
pub static BEDIT: &[u8] = include_bytes!("../../assets/icons/attedit.svg");
/// Blend curves (`blend`).
pub static BLEND: &[u8] = include_bytes!("../../assets/icons/v2/blend.svg");
/// Insert (`block`).
pub static BLOCK: &[u8] = include_bytes!("../../assets/icons/v2/block.svg");
/// Select Objects (`cbar`).
pub static CBAR: &[u8] = include_bytes!("../../assets/icons/v2/cbar.svg");
/// Hide All (`cbarHide`).
pub static CBAR_HIDE: &[u8] = include_bytes!("../../assets/icons/v2/cbarHide.svg");
/// Show All (`cbarShow`).
pub static CBAR_SHOW: &[u8] = include_bytes!("../../assets/icons/v2/cbarShow.svg");
/// Copy with Base Point (`cbase`).
pub static CBASE: &[u8] = include_bytes!("../../assets/icons/v2/cbase.svg");
/// Copy (`cclip`).
pub static CCLIP: &[u8] = include_bytes!("../../assets/icons/v2/cclip.svg");
/// Centerline (`center`).
pub static CENTER: &[u8] = include_bytes!("../../assets/icons/v2/center.svg");
/// Change Space (`chspace`).
pub static CHSPACE: &[u8] = include_bytes!("../../assets/icons/v2/chspace.svg");
/// Clean Screen (`clean`).
pub static CLEAN: &[u8] = include_bytes!("../../assets/icons/v2/clean.svg");
/// Underlay (`clip`).
pub static CLIP: &[u8] = include_bytes!("../../assets/icons/v2/clip.svg");
/// Close All (`closeAll`).
pub static CLOSE_ALL: &[u8] = include_bytes!("../../assets/icons/v2/closeAll.svg");
/// Close (`closeDoc`).
pub static CLOSE_DOC: &[u8] = include_bytes!("../../assets/icons/v2/closeDoc.svg");
/// Color (`color`).
pub static COLOR: &[u8] = include_bytes!("../../assets/icons/v2/color.svg");
/// DGN Underlay (`dgnRef`).
pub static DGN_REF: &[u8] = include_bytes!("../../assets/icons/v2/dgnRef.svg");
/// Diameter (`diameter`).
pub static DIAMETER: &[u8] = include_bytes!("../../assets/icons/v2/diameter.svg");
/// Dimension (`dim`).
pub static DIM: &[u8] = include_bytes!("../../assets/icons/v2/dim.svg");
/// Override (`dimOverride`).
pub static DIM_OVERRIDE: &[u8] = include_bytes!("../../assets/icons/v2/dimOverride.svg");
/// Arc Length (`dimarc`).
pub static DIMARC: &[u8] = include_bytes!("../../assets/icons/v2/dimarc.svg");
/// Jogged (`dimjogged`).
pub static DIMJOGGED: &[u8] = include_bytes!("../../assets/icons/v2/dimjogged.svg");
/// Reassociate Dimensions (`dimreassoc`).
pub static DIMREASSOC: &[u8] = include_bytes!("../../assets/icons/v2/dimreassoc.svg");
/// Dimension Style (`dimstyle`).
pub static DIMSTYLE: &[u8] = include_bytes!("../../assets/icons/v2/dimstyle.svg");
/// Update (`dimupd`).
pub static DIMUPD: &[u8] = include_bytes!("../../assets/icons/v2/dimupd.svg");
/// Bring Above Objects (`doAbove`).
pub static DO_ABOVE: &[u8] = include_bytes!("../../assets/icons/v2/doAbove.svg");
/// Send to Back (`doBack`).
pub static DO_BACK: &[u8] = include_bytes!("../../assets/icons/v2/doBack.svg");
/// Bring to Front (`doFront`).
pub static DO_FRONT: &[u8] = include_bytes!("../../assets/icons/v2/doFront.svg");
/// Send Under Objects (`doUnder`).
pub static DO_UNDER: &[u8] = include_bytes!("../../assets/icons/v2/doUnder.svg");
/// Draw order (`draworder`).
pub static DRAWORDER: &[u8] = include_bytes!("../../assets/icons/v2/draworder.svg");
/// Drafting Settings (`dsettings`).
pub static DSETTINGS: &[u8] = include_bytes!("../../assets/icons/v2/dsettings.svg");
/// Text Angle (`dtA`).
pub static DT_A: &[u8] = include_bytes!("../../assets/icons/v2/dtA.svg");
/// Center Justify (`dtC`).
pub static DT_C: &[u8] = include_bytes!("../../assets/icons/v2/dtC.svg");
/// Home (`dtH`).
pub static DT_H: &[u8] = include_bytes!("../../assets/icons/v2/dtH.svg");
/// Left Justify (`dtL`).
pub static DT_L: &[u8] = include_bytes!("../../assets/icons/v2/dtL.svg");
/// Right Justify (`dtR`).
pub static DT_R: &[u8] = include_bytes!("../../assets/icons/v2/dtR.svg");
/// DWF Underlay (`dwfRef`).
pub static DWF_REF: &[u8] = include_bytes!("../../assets/icons/v2/dwfRef.svg");
/// Drawing Properties (`dwgprops`).
pub static DWGPROPS: &[u8] = include_bytes!("../../assets/icons/v2/dwgprops.svg");
/// Select Objects (`dynDim`).
pub static DYN_DIM: &[u8] = include_bytes!("../../assets/icons/v2/dynDim.svg");
/// Hide All (`dynHide`).
pub static DYN_HIDE: &[u8] = include_bytes!("../../assets/icons/v2/dynHide.svg");
/// Show All (`dynShow`).
pub static DYN_SHOW: &[u8] = include_bytes!("../../assets/icons/v2/dynShow.svg");
/// Arc (`ellArc`).
pub static ELL_ARC: &[u8] = include_bytes!("../../assets/icons/v2/ellArc.svg");
/// Center (`ellC`).
pub static ELL_C: &[u8] = include_bytes!("../../assets/icons/v2/ellC.svg");
/// Ellipse (`ellipse`).
pub static ELLIPSE: &[u8] = include_bytes!("../../assets/icons/v2/ellipse.svg");
/// eTransmit (`etransmit`).
pub static ETRANSMIT: &[u8] = include_bytes!("../../assets/icons/v2/etransmit.svg");
/// Save as DXF (`export`).
pub static EXPORT: &[u8] = include_bytes!("../../assets/icons/v2/export.svg");
/// Extract data (`extract`).
pub static EXTRACT: &[u8] = include_bytes!("../../assets/icons/v2/extract.svg");
/// Field (`field`).
pub static FIELD: &[u8] = include_bytes!("../../assets/icons/v2/field.svg");
/// Find (`find`).
pub static FIND: &[u8] = include_bytes!("../../assets/icons/v2/find.svg");
/// Freeze layer (`freeze`).
pub static FREEZE: &[u8] = include_bytes!("../../assets/icons/v2/freeze.svg");
/// Send Hatches to Back (`hatchBack`).
pub static HATCH_BACK: &[u8] = include_bytes!("../../assets/icons/v2/hatchBack.svg");
/// Hidden (`hidden`).
pub static HIDDEN: &[u8] = include_bytes!("../../assets/icons/v2/hidden.svg");
/// Point (`idpt`).
pub static IDPT: &[u8] = include_bytes!("../../assets/icons/v2/idpt.svg");
/// Raster image (`image`).
pub static IMAGE: &[u8] = include_bytes!("../../assets/icons/v2/image.svg");
/// Image (`imgclip`).
pub static IMGCLIP: &[u8] = include_bytes!("../../assets/icons/v2/imgclip.svg");
/// Inspection (`inspect`).
pub static INSPECT: &[u8] = include_bytes!("../../assets/icons/v2/inspect.svg");
/// NE Isometric (`isoNE`).
pub static ISO_NE: &[u8] = include_bytes!("../../assets/icons/v2/isoNE.svg");
/// NW Isometric (`isoNW`).
pub static ISO_NW: &[u8] = include_bytes!("../../assets/icons/v2/isoNW.svg");
/// SE Isometric (`isoSE`).
pub static ISO_SE: &[u8] = include_bytes!("../../assets/icons/v2/isoSE.svg");
/// SW Isometric (`isoSW`).
pub static ISO_SW: &[u8] = include_bytes!("../../assets/icons/v2/isoSW.svg");
/// Jog Line (`jog`).
pub static JOG: &[u8] = include_bytes!("../../assets/icons/v2/jog.svg");
/// Justify (`justify`).
pub static JUSTIFY: &[u8] = include_bytes!("../../assets/icons/v2/justify.svg");
/// Layer Delete (`layDel`).
pub static LAY_DEL: &[u8] = include_bytes!("../../assets/icons/v2/layDel.svg");
/// Layer Merge (`layMerge`).
pub static LAY_MERGE: &[u8] = include_bytes!("../../assets/icons/v2/layMerge.svg");
/// Turn All Layers On (`layOn`).
pub static LAY_ON: &[u8] = include_bytes!("../../assets/icons/v2/layOn.svg");
/// Turn layer off (`layoff`).
pub static LAYOFF: &[u8] = include_bytes!("../../assets/icons/v2/layoff.svg");
/// Multileader (`leader`).
pub static LEADER: &[u8] = include_bytes!("../../assets/icons/v2/leader.svg");
/// Drawing Limits (`limits`).
pub static LIMITS: &[u8] = include_bytes!("../../assets/icons/v2/limits.svg");
/// Line (`line`).
pub static LINE: &[u8] = include_bytes!("../../assets/icons/v2/line.svg");
/// Linear (`linear`).
pub static LINEAR: &[u8] = include_bytes!("../../assets/icons/v2/linear.svg");
/// Linetype (`linetype`).
pub static LINETYPE: &[u8] = include_bytes!("../../assets/icons/v2/linetype.svg");
/// Import (`import`).
pub static IMPORT: &[u8] = include_bytes!("../../assets/icons/import_obj.svg");
/// Hyperlink (`link`).
pub static LINK: &[u8] = include_bytes!("../../assets/icons/v2/link.svg");
/// List (`list`).
pub static LIST: &[u8] = include_bytes!("../../assets/icons/v2/list.svg");
/// Lock layer (`lock`).
pub static LOCK: &[u8] = include_bytes!("../../assets/icons/v2/lock.svg");
/// Lineweight (`lweight`).
pub static LWEIGHT: &[u8] = include_bytes!("../../assets/icons/v2/lweight.svg");
/// Region/Mass Properties (`massprop`).
pub static MASSPROP: &[u8] = include_bytes!("../../assets/icons/v2/massprop.svg");
/// Measure (`measure`).
pub static MEASURE: &[u8] = include_bytes!("../../assets/icons/v2/measure.svg");
/// Measure (`measure2`).
pub static MEASURE2: &[u8] = include_bytes!("../../assets/icons/v2/measure2.svg");
/// Area (`mgArea`).
pub static MG_AREA: &[u8] = include_bytes!("../../assets/icons/v2/mgArea.svg");
/// Distance (`mgDist`).
pub static MG_DIST: &[u8] = include_bytes!("../../assets/icons/v2/mgDist.svg");
/// Volume (`mgVol`).
pub static MG_VOL: &[u8] = include_bytes!("../../assets/icons/v2/mgVol.svg");
/// Multiline (`mledit`).
pub static MLEDIT: &[u8] = include_bytes!("../../assets/icons/v2/mledit.svg");
/// Multiline (`mline`).
pub static MLINE: &[u8] = include_bytes!("../../assets/icons/v2/mline.svg");
/// Multiline Style (`mlinestyle`).
pub static MLINESTYLE: &[u8] = include_bytes!("../../assets/icons/v2/mlinestyle.svg");
/// Multileader Style (`mlstyle`).
pub static MLSTYLE: &[u8] = include_bytes!("../../assets/icons/v2/mlstyle.svg");
/// Multiple Point (`mpoint`).
pub static MPOINT: &[u8] = include_bytes!("../../assets/icons/v2/mpoint.svg");
/// New (`new`).
pub static NEW: &[u8] = include_bytes!("../../assets/icons/v2/new.svg");
/// Oblique (`oblique`).
pub static OBLIQUE: &[u8] = include_bytes!("../../assets/icons/v2/oblique.svg");
/// OLE Object (`ole`).
pub static OLE: &[u8] = include_bytes!("../../assets/icons/v2/ole.svg");
/// OLE Links (`oleLinks`).
pub static OLE_LINKS: &[u8] = include_bytes!("../../assets/icons/v2/oleLinks.svg");
/// Open… (`open`).
pub static OPEN: &[u8] = include_bytes!("../../assets/icons/v2/open.svg");
/// Pan (`pan`).
pub static PAN: &[u8] = include_bytes!("../../assets/icons/v2/pan.svg");
/// Down (`panD`).
pub static PAN_D: &[u8] = include_bytes!("../../assets/icons/v2/panD.svg");
/// Left (`panL`).
pub static PAN_L: &[u8] = include_bytes!("../../assets/icons/v2/panL.svg");
/// Point (`panPt`).
pub static PAN_PT: &[u8] = include_bytes!("../../assets/icons/v2/panPt.svg");
/// Right (`panR`).
pub static PAN_R: &[u8] = include_bytes!("../../assets/icons/v2/panR.svg");
/// Up (`panU`).
pub static PAN_U: &[u8] = include_bytes!("../../assets/icons/v2/panU.svg");
/// Paste (`paste`).
pub static PASTE: &[u8] = include_bytes!("../../assets/icons/v2/paste.svg");
/// Paste as Block (`pasteBlk`).
pub static PASTE_BLK: &[u8] = include_bytes!("../../assets/icons/v2/pasteBlk.svg");
/// Paste to Original Coordinates (`pasteOrig`).
pub static PASTE_ORIG: &[u8] = include_bytes!("../../assets/icons/v2/pasteOrig.svg");
/// Paste Special (`pasteSpec`).
pub static PASTE_SPEC: &[u8] = include_bytes!("../../assets/icons/v2/pasteSpec.svg");
/// PDF import (`pdf`).
pub static PDF: &[u8] = include_bytes!("../../assets/icons/v2/pdf.svg");
/// Plan View (`plan`).
pub static PLAN: &[u8] = include_bytes!("../../assets/icons/v2/plan.svg");
/// Plot (`plot`).
pub static PLOT: &[u8] = include_bytes!("../../assets/icons/plot.svg");
/// Plot Style Manager (`plotstyles`).
pub static PLOTSTYLES: &[u8] = include_bytes!("../../assets/icons/v2/plotstyles.svg");
/// 3D Polyline (`poly3d`).
pub static POLY3D: &[u8] = include_bytes!("../../assets/icons/v2/poly3d.svg");
/// Named views (`preview`).
pub static PREVIEW: &[u8] = include_bytes!("../../assets/icons/v2/preview.svg");
/// List (`props`).
pub static PROPS: &[u8] = include_bytes!("../../assets/icons/properties.svg");
/// Plot Style (`pstyle`).
pub static PSTYLE: &[u8] = include_bytes!("../../assets/icons/v2/pstyle.svg");
/// Point Style (`ptype`).
pub static PTYPE: &[u8] = include_bytes!("../../assets/icons/v2/ptype.svg");
/// Exit (`quit`).
pub static QUIT: &[u8] = include_bytes!("../../assets/icons/v2/quit.svg");
/// Radius (`radius`).
pub static RADIUS: &[u8] = include_bytes!("../../assets/icons/v2/radius.svg");
/// Rectangle (`rect`).
pub static RECT: &[u8] = include_bytes!("../../assets/icons/v2/rect.svg");
/// Redo (`redo`).
pub static REDO: &[u8] = include_bytes!("../../assets/icons/v2/redo.svg");
/// Redraw (`redraw`).
pub static REDRAW: &[u8] = include_bytes!("../../assets/icons/v2/redraw.svg");
/// Regen (`regen`).
pub static REGEN: &[u8] = include_bytes!("../../assets/icons/v2/regen.svg");
/// Regen All (`regenall`).
pub static REGENALL: &[u8] = include_bytes!("../../assets/icons/v2/regenall.svg");
/// Region (`region`).
pub static REGION: &[u8] = include_bytes!("../../assets/icons/v2/region.svg");
/// Rename (`rename`).
pub static RENAME: &[u8] = include_bytes!("../../assets/icons/v2/rename.svg");
/// Reverse (`reverse`).
pub static REVERSE: &[u8] = include_bytes!("../../assets/icons/v2/reverse.svg");
/// Save (`save`).
pub static SAVE: &[u8] = include_bytes!("../../assets/icons/v2/save.svg");
/// Save as… (`saveas`).
pub static SAVEAS: &[u8] = include_bytes!("../../assets/icons/v2/saveas.svg");
/// Scale List (`scalelist`).
pub static SCALELIST: &[u8] = include_bytes!("../../assets/icons/v2/scalelist.svg");
/// Scale text (`scaletext`).
pub static SCALETEXT: &[u8] = include_bytes!("../../assets/icons/v2/scaletext.svg");
/// Run script (`script`).
pub static SCRIPT: &[u8] = include_bytes!("../../assets/icons/v2/script.svg");
/// Select All (`selall`).
pub static SELALL: &[u8] = include_bytes!("../../assets/icons/v2/selall.svg");
/// Select similar (`selsim`).
pub static SELSIM: &[u8] = include_bytes!("../../assets/icons/v2/selsim.svg");
/// Options (`settings`).
pub static SETTINGS: &[u8] = include_bytes!("../../assets/icons/v2/settings.svg");
/// Set Variable (`setvar`).
pub static SETVAR: &[u8] = include_bytes!("../../assets/icons/v2/setvar.svg");
/// Shaded (`shaded`).
pub static SHADED: &[u8] = include_bytes!("../../assets/icons/v2/shaded.svg");
/// New Sheet Set (`sheetNew`).
pub static SHEET_NEW: &[u8] = include_bytes!("../../assets/icons/v2/sheetNew.svg");
/// Open Sheet Set (`sheetOpen`).
pub static SHEET_OPEN: &[u8] = include_bytes!("../../assets/icons/v2/sheetOpen.svg");
/// Keyboard Shortcuts (`shortcuts`).
pub static SHORTCUTS: &[u8] = include_bytes!("../../assets/icons/v2/shortcuts.svg");
/// Spelling (`spell`).
pub static SPELL: &[u8] = include_bytes!("../../assets/icons/v2/spell.svg");
/// Spline (`spline`).
pub static SPLINE: &[u8] = include_bytes!("../../assets/icons/spline.svg");
/// Layer states (`states`).
pub static STATES: &[u8] = include_bytes!("../../assets/icons/v2/states.svg");
/// Status (`status`).
pub static STATUS: &[u8] = include_bytes!("../../assets/icons/v2/status.svg");
/// Table (`table`).
pub static TABLE: &[u8] = include_bytes!("../../assets/icons/v2/table.svg");
/// Table Style (`tblstyle`).
pub static TBLSTYLE: &[u8] = include_bytes!("../../assets/icons/v2/tblstyle.svg");
/// Thickness (`thickness`).
pub static THICKNESS: &[u8] = include_bytes!("../../assets/icons/v2/thickness.svg");
/// Transparency (`transp`).
pub static TRANSP: &[u8] = include_bytes!("../../assets/icons/v2/transp.svg");
/// Bring Dimensions to Front (`ttfDim`).
pub static TTF_DIM: &[u8] = include_bytes!("../../assets/icons/v2/ttfDim.svg");
/// Bring Leaders to Front (`ttfLead`).
pub static TTF_LEAD: &[u8] = include_bytes!("../../assets/icons/v2/ttfLead.svg");
/// Bring Text to Front (`ttfText`).
pub static TTF_TEXT: &[u8] = include_bytes!("../../assets/icons/v2/ttfText.svg");
/// 3 Point (`ucs3`).
pub static UCS3: &[u8] = include_bytes!("../../assets/icons/v2/ucs3.svg");
/// Origin (`ucsO`).
pub static UCS_O: &[u8] = include_bytes!("../../assets/icons/v2/ucsO.svg");
/// Object (`ucsOb`).
pub static UCS_OB: &[u8] = include_bytes!("../../assets/icons/v2/ucsOb.svg");
/// View (`ucsV`).
pub static UCS_V: &[u8] = include_bytes!("../../assets/icons/v2/ucsV.svg");
/// World (`ucsW`).
pub static UCS_W: &[u8] = include_bytes!("../../assets/icons/v2/ucsW.svg");
/// Undo (`undo`).
pub static UNDO: &[u8] = include_bytes!("../../assets/icons/v2/undo.svg");
/// Units (`units`).
pub static UNITS: &[u8] = include_bytes!("../../assets/icons/v2/units.svg");
/// Layer Unlock (`unlock`).
pub static UNLOCK: &[u8] = include_bytes!("../../assets/icons/v2/unlock.svg");
/// Update Fields (`updfield`).
pub static UPDFIELD: &[u8] = include_bytes!("../../assets/icons/v2/updfield.svg");
/// Back (`vBack`).
pub static V_BACK: &[u8] = include_bytes!("../../assets/icons/v2/vBack.svg");
/// Bottom (`vBottom`).
pub static V_BOTTOM: &[u8] = include_bytes!("../../assets/icons/v2/vBottom.svg");
/// Left (`vLeft`).
pub static V_LEFT: &[u8] = include_bytes!("../../assets/icons/v2/vLeft.svg");
/// Viewports (`viewports`).
pub static VIEWPORTS: &[u8] = include_bytes!("../../assets/icons/v2/viewports.svg");
/// Named Views (`views`).
pub static VIEWS: &[u8] = include_bytes!("../../assets/icons/v2/views.svg");
/// Viewport (`vpclip`).
pub static VPCLIP: &[u8] = include_bytes!("../../assets/icons/v2/vpclip.svg");
/// 2D Wireframe (`vs2d`).
pub static VS2D: &[u8] = include_bytes!("../../assets/icons/v2/vs2d.svg");
/// Conceptual (`vsConcept`).
pub static VS_CONCEPT: &[u8] = include_bytes!("../../assets/icons/v2/vsConcept.svg");
/// Realistic (`vsReal`).
pub static VS_REAL: &[u8] = include_bytes!("../../assets/icons/v2/vsReal.svg");
/// Visual Styles Manager (`vstyles`).
pub static VSTYLES: &[u8] = include_bytes!("../../assets/icons/v2/vstyles.svg");
/// Write Block (`wblock`).
pub static WBLOCK: &[u8] = include_bytes!("../../assets/icons/v2/wblock.svg");
/// Wireframe (`wire`).
pub static WIRE: &[u8] = include_bytes!("../../assets/icons/v2/wire.svg");
/// Xref (`xclip`).
pub static XCLIP: &[u8] = include_bytes!("../../assets/icons/v2/xclip.svg");
/// Data link (`xref`).
pub static XREF: &[u8] = include_bytes!("../../assets/icons/v2/xref.svg");
/// All (`zAll`).
pub static Z_ALL: &[u8] = include_bytes!("../../assets/icons/v2/zAll.svg");
/// Center (`zCenter`).
pub static Z_CENTER: &[u8] = include_bytes!("../../assets/icons/v2/zCenter.svg");
/// Dynamic (`zDyn`).
pub static Z_DYN: &[u8] = include_bytes!("../../assets/icons/v2/zDyn.svg");
/// In (`zIn`).
pub static Z_IN: &[u8] = include_bytes!("../../assets/icons/v2/zIn.svg");
/// Object (`zObj`).
pub static Z_OBJ: &[u8] = include_bytes!("../../assets/icons/v2/zObj.svg");
/// Out (`zOut`).
pub static Z_OUT: &[u8] = include_bytes!("../../assets/icons/v2/zOut.svg");
/// Previous (`zPrev`).
pub static Z_PREV: &[u8] = include_bytes!("../../assets/icons/v2/zPrev.svg");
/// Scale (`zScale`).
pub static Z_SCALE: &[u8] = include_bytes!("../../assets/icons/v2/zScale.svg");
/// Extents (`zoomExt`).
pub static ZOOM_EXT: &[u8] = include_bytes!("../../assets/icons/zoom_ext.svg");
/// Find & replace (`zoomWin`).
pub static ZOOM_WIN: &[u8] = include_bytes!("../../assets/icons/v2/zoomWin.svg");

/// Command lines (upper case, as the menu bar and the ribbon run them) and
/// their icon. [`super::registry::command_icon`] looks here before the
/// ribbon, so an option line gets its own icon and not its verb's.
pub const COMMAND_LINE_ICONS: &[(&str, &[u8])] = &[
    ("NEW", NEW),
    ("QNEW", NEW),
    ("OPEN", OPEN),
    ("SAVE", SAVE),
    ("QSAVE", SAVE),
    ("SAVEAS", SAVEAS),
    ("EXPORT", EXPORT),
    ("STLOUT", EXPORT),
    ("STEPOUT", EXPORT),
    ("ETRANSMIT", ETRANSMIT),
    ("EXPORTPDF", PDF),
    ("CLOSE", CLOSE_DOC),
    ("CLOSEALL", CLOSE_ALL),
    ("QUIT", QUIT),
    ("EXIT", QUIT),
    ("U", UNDO),
    ("UNDO", UNDO),
    ("REDO", REDO),
    ("MREDO", REDO),
    ("PLOT", PLOT),
    ("PRINT", PLOT),
    ("PUBLISH", PLOT),
    ("DWGPROPS", DWGPROPS),
    ("NEWSHEETSET", SHEET_NEW),
    ("OPENSHEETSET", SHEET_OPEN),
    ("STYLESMANAGER", PLOTSTYLES),
    ("STATUS", STATUS),
    ("COPYCLIP", CCLIP),
    ("COPYBASE", CBASE),
    ("PASTECLIP", PASTE),
    ("PASTE", PASTE),
    ("PASTEBLOCK", PASTE_BLK),
    ("PASTEORIG", PASTE_ORIG),
    ("PASTESPEC", PASTE_SPEC),
    ("AI_SELALL", SELALL),
    ("SELECTALL", SELALL),
    ("SELECTSIMILAR", SELSIM),
    ("OLELINKS", OLE_LINKS),
    ("FIND", FIND),
    ("REDRAW", REDRAW),
    ("REDRAWALL", REDRAW),
    ("REGEN", REGEN),
    ("REGENALL", REGENALL),
    ("ZOOM P", Z_PREV),
    ("ZOOM W", ZOOM_WIN),
    ("ZOOM D", Z_DYN),
    ("ZOOM S", Z_SCALE),
    ("ZOOM C", Z_CENTER),
    ("ZOOM O", Z_OBJ),
    ("ZOOM 2X", Z_IN),
    ("ZOOM .5X", Z_OUT),
    ("ZOOM A", Z_ALL),
    ("ZOOM E", ZOOM_EXT),
    ("PAN", PAN),
    ("-PAN", PAN_PT),
    ("PAN LEFT", PAN_L),
    ("PAN RIGHT", PAN_R),
    ("PAN UP", PAN_U),
    ("PAN DOWN", PAN_D),
    ("CLEANSCREEN", CLEAN),
    ("CLEANSCREENON", CLEAN),
    ("VPORTS", VIEWPORTS),
    ("VIEW", VIEWS),
    ("PLAN", PLAN),
    ("-VIEW BOTTOM", V_BOTTOM),
    ("-VIEW LEFT", V_LEFT),
    ("-VIEW BACK", V_BACK),
    ("-VIEW SWISO", ISO_SW),
    ("-VIEW SEISO", ISO_SE),
    ("-VIEW NEISO", ISO_NE),
    ("-VIEW NWISO", ISO_NW),
    ("VSCURRENT 2", VS2D),
    ("VSCURRENT W", WIRE),
    ("VSCURRENT H", HIDDEN),
    ("VSCURRENT R", VS_REAL),
    ("VSCURRENT C", VS_CONCEPT),
    ("VSCURRENT S", SHADED),
    ("VISUALSTYLES", VSTYLES),
    ("ATTDISP", ATTDISP),
    ("INSERT", BLOCK),
    ("XATTACH", XREF),
    ("ATTACH", XREF),
    ("EXTERNALREFERENCES", XREF),
    ("DATALINK", XREF),
    ("DWFATTACH", DWF_REF),
    ("DGNATTACH", DGN_REF),
    ("PDFATTACH", PDF),
    ("PDFIMPORT", PDF),
    ("IMAGEATTACH", IMAGE),
    ("FIELD", FIELD),
    ("INSERTOBJ", OLE),
    ("IMPORTOBJ", IMPORT),
    ("HYPERLINK", LINK),
    ("LAYERSTATE", STATES),
    ("LAYWALK", STATES),
    ("LAYOFF", LAYOFF),
    ("LAYON", LAY_ON),
    ("LAYFRZ", FREEZE),
    ("LAYTHW", FREEZE),
    ("LAYLCK", LOCK),
    ("LAYULK", UNLOCK),
    ("LAYMRG", LAY_MERGE),
    ("LAYDEL", LAY_DEL),
    ("COLOR", COLOR),
    ("LINETYPE", LINETYPE),
    ("LWEIGHT", LWEIGHT),
    ("TRANSPARENCY", TRANSP),
    ("DIMSTYLE", DIMSTYLE),
    ("TABLESTYLE", TBLSTYLE),
    ("MLEADERSTYLE", MLSTYLE),
    ("PLOTSTYLE", PSTYLE),
    ("PTYPE", PTYPE),
    ("MLSTYLE", MLINESTYLE),
    ("UNITS", UNITS),
    ("THICKNESS", THICKNESS),
    ("LIMITS", LIMITS),
    ("RENAME", RENAME),
    ("PROPERTIES", PROPS),
    ("SPELL", SPELL),
    ("DRAWORDER", DRAWORDER),
    ("DRAWORDER F", DO_FRONT),
    ("DRAWORDER B", DO_BACK),
    ("DRAWORDER A", DO_ABOVE),
    ("DRAWORDER U", DO_UNDER),
    ("DRAWORDER_FRONT", DO_FRONT),
    ("DRAWORDER_BACK", DO_BACK),
    ("DRAWORDER_ABOVE", DO_ABOVE),
    ("DRAWORDER_UNDER", DO_UNDER),
    ("TEXTTOFRONT T", TTF_TEXT),
    ("TEXTTOFRONT D", TTF_DIM),
    ("TEXTTOFRONT L", TTF_LEAD),
    ("HATCHTOBACK", HATCH_BACK),
    ("MEASUREGEOM", MEASURE),
    ("MEASUREGEOM D", MG_DIST),
    ("MEASUREGEOM R", RADIUS),
    ("MEASUREGEOM A", ANGULAR),
    ("MEASUREGEOM AR", MG_AREA),
    ("MEASUREGEOM V", MG_VOL),
    ("DIST", MG_DIST),
    ("AREA", MG_AREA),
    ("MASSPROP", MASSPROP),
    ("LIST", LIST),
    ("ID", IDPT),
    ("SETVAR", SETVAR),
    ("UPDATEFIELD", UPDFIELD),
    ("PLUGINS", APP),
    ("SCRIPT", SCRIPT),
    ("UCS W", UCS_W),
    ("UCS O", UCS_O),
    ("UCS 3", UCS3),
    ("UCS OB", UCS_OB),
    ("UCS V", UCS_V),
    ("DSETTINGS", DSETTINGS),
    ("OPTIONS", SETTINGS),
    ("CONSTRAINTSETTINGS", SETTINGS),
    ("LINE", LINE),
    ("MLINE", MLINE),
    ("3DPOLY", POLY3D),
    ("ARC C", ACONT),
    ("CIRCLE D", DIAMETER),
    ("SPLINE F", SPLINE),
    ("ELLIPSE", ELLIPSE),
    ("ELLIPSE C", ELL_C),
    ("ELLIPSE A", ELL_ARC),
    ("WBLOCK", WBLOCK),
    ("TABLE", TABLE),
    ("POINT", IDPT),
    ("POINT M", MPOINT),
    ("MEASURE", MEASURE2),
    ("REGION", REGION),
    ("BOUNDARY", REGION),
    ("DIM", DIM),
    ("DIMLINEAR", LINEAR),
    ("DIMARC", DIMARC),
    ("DIMJOGGED", DIMJOGGED),
    ("DIMDIAMETER", DIAMETER),
    ("DIMRADIUS", RADIUS),
    ("DIMANGULAR", ANGULAR),
    ("MLEADER", LEADER),
    ("DIMCENTER", CENTER),
    ("CENTERLINE", CENTER),
    ("DIMINSPECT", INSPECT),
    ("DIMJOGLINE", JOG),
    ("DIMEDIT O", OBLIQUE),
    ("DIMTEDIT", DT_H),
    ("DIMTEDIT H", DT_H),
    ("DIMTEDIT A", DT_A),
    ("DIMTEDIT L", DT_L),
    ("DIMTEDIT C", DT_C),
    ("DIMTEDIT R", DT_R),
    ("DIMOVERRIDE", DIM_OVERRIDE),
    ("-DIMSTYLE A", DIMUPD),
    ("DIMREASSOCIATE", DIMREASSOC),
    ("MLEDIT", MLEDIT),
    ("JUSTIFYTEXT", JUSTIFY),
    ("SCALETEXT", SCALETEXT),
    ("XCLIP", XCLIP),
    ("IMAGECLIP", IMGCLIP),
    ("CLIP", CLIP),
    ("VPCLIP", VPCLIP),
    ("RECTANG", RECT),
    ("RECT", RECT),
    ("REVERSE", REVERSE),
    ("BLEND", BLEND),
    ("CHSPACE", CHSPACE),
    ("SCALELISTEDIT", SCALELIST),
    ("DATAEXTRACTION", EXTRACT),
    ("BEDIT", BEDIT),
    ("GCFIX", LOCK),
    ("DCHORIZONTAL", LINEAR),
    ("DCANGULAR", ANGULAR),
    ("DCRADIUS", RADIUS),
    ("DCDIAMETER", DIAMETER),
    ("CONSTRAINTBAR", CBAR),
    ("CONSTRAINTBAR S", CBAR_SHOW),
    ("CONSTRAINTBAR H", CBAR_HIDE),
    ("DCDISPLAY", DYN_DIM),
    ("DCDISPLAY S", DYN_SHOW),
    ("DCDISPLAY H", DYN_HIDE),
    ("SHORTCUTS", SHORTCUTS),
    ("ALIASEDIT", SHORTCUTS),
];

/// The icon of command line `line` from [`COMMAND_LINE_ICONS`].
pub fn command_line_icon(line: &str) -> Option<&'static [u8]> {
    static BY_LINE: std::sync::OnceLock<std::collections::HashMap<&'static str, &'static [u8]>> =
        std::sync::OnceLock::new();
    let by_line = BY_LINE.get_or_init(|| COMMAND_LINE_ICONS.iter().copied().collect());
    if line.bytes().any(|b| b.is_ascii_lowercase()) {
        by_line.get(line.to_ascii_uppercase().as_str()).copied()
    } else {
        by_line.get(line).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::registry::{command_icon, ribbon_commands};

    /// The design hand-off: per icon, the commands it stands for (see
    /// `PHASE3_ICONS.md`, section 1).
    const MAPPING: &str = include_str!("../../assets/icons/v2/mapping.csv");

    /// Where the new icon set deliberately departs from `mapping.csv`'s
    /// `commands` column: the command takes another design icon that fits
    /// it better, or (`None`) keeps its OCS icon because the design has none
    /// for it.
    const DEPARTURES: &[(&str, &str, Option<&str>)] = &[
        ("text", "SPELL", Some("spell")),
        ("text", "FIELD", Some("field")),
        ("text", "ALIASEDIT", Some("shortcuts")),
        ("text", "ATTDEF", None),
        ("props", "LIST", Some("list")),
        ("preview", "VIEW", Some("views")),
        ("lengthen", "DIST", Some("mgDist")),
        ("region", "AREA", Some("mgArea")),
        ("reverse", "ATTSYNC", None),
        ("reverse", "DIMSTYLE", Some("dimstyle")),
        ("xref", "DIMREASSOCIATE", Some("dimreassoc")),
        ("layers", "LAYMRG", Some("layMerge")),
        ("erase", "LAYDEL", Some("layDel")),
        ("layoff", "LAYON", Some("layOn")),
        ("lock", "LAYULK", Some("unlock")),
        ("scale", "ADJUST", None),
        ("table", "INSERTOBJ", Some("ole")),
        ("check", "ESC", None),
        ("block", "IMPORTOBJ", Some("import")),
        ("rect", "CLIP", Some("clip")),
        // PHASE3_ICONS.md: ZOOM P takes the zPrev icon, not `undo`.
        ("undo", "ZOOM P", Some("zPrev")),
        ("zoomWin", "FIND", Some("find")),
        // The ZOOM verb takes its default option's icon: OCS's ZOOM entry
        // is Zoom Extents. OCS has no separate realtime zoom entry.
        ("zReal", "ZOOM", Some("zoomExt")),
    ];

    fn rows() -> Vec<Vec<String>> {
        // A small CSV reader: quoted fields may hold commas.
        MAPPING
            .lines()
            .skip(1)
            .filter(|line| !line.is_empty())
            .map(|line| {
                let mut fields = vec![String::new()];
                let mut quoted = false;
                for ch in line.chars() {
                    match ch {
                        '"' => quoted = !quoted,
                        ',' if !quoted => fields.push(String::new()),
                        _ => fields.last_mut().unwrap().push(ch),
                    }
                }
                fields
            })
            .collect()
    }

    /// The new artwork of design icon `key`: its `v2` file, or the OCS file
    /// that took its drawing over.
    fn artwork(key: &str, ocs_files: &str) -> Vec<u8> {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/icons/");
        if let Ok(bytes) = std::fs::read(format!("{root}v2/{key}.svg")) {
            return bytes;
        }
        ocs_files
            .split_whitespace()
            .filter_map(|file| std::fs::read(format!("{root}{file}")).ok())
            .find(|bytes| String::from_utf8_lossy(bytes).contains(r#"viewBox="2.5 2.5 19 19""#))
            .unwrap_or_else(|| panic!("no new artwork for design icon {key}"))
    }

    fn known_commands() -> std::collections::HashSet<String> {
        let mut known: std::collections::HashSet<String> =
            ribbon_commands().keys().cloned().collect();
        known.extend(COMMAND_LINE_ICONS.iter().map(|(c, _)| c.to_string()));
        known.extend(
            crate::ui::menu_bar::search_entries()
                .iter()
                .map(|e| e.line.to_string()),
        );
        known
    }

    /// The `commands` column holds command lines separated by spaces; a
    /// line may itself hold a space (`ARC SCE`). Longest known line first.
    fn split_commands(text: &str, known: &std::collections::HashSet<String>) -> Vec<String> {
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut out = Vec::new();
        let mut i = 0;
        while i < words.len() {
            let n = (1..=3.min(words.len() - i))
                .rev()
                .find(|&n| n == 1 || known.contains(&words[i..i + n].join(" ")))
                .unwrap_or(1);
            out.push(words[i..i + n].join(" "));
            i += n;
        }
        out
    }

    fn exists_in_ocs(command: &str) -> bool {
        let upper = command.to_ascii_uppercase();
        crate::ui::menu_bar::implemented(&upper)
            || ribbon_commands().contains_key(&upper)
            || upper
                .strip_prefix('-')
                .is_some_and(|base| ribbon_commands().contains_key(base))
    }

    #[test]
    fn every_mapped_command_shows_its_new_icon() {
        let rows = rows();
        let known = known_commands();
        let by_key: std::collections::HashMap<&str, &Vec<String>> =
            rows.iter().map(|row| (row[0].as_str(), row)).collect();
        let mut checked = 0;
        let mut failures = Vec::new();
        for row in &rows {
            let (key, commands, ocs_files, source) = (&row[0], &row[3], &row[4], &row[5]);
            if !["exact", "variant", "label"]
                .iter()
                .any(|s| source.contains(s))
            {
                continue;
            }
            for command in split_commands(commands, &known) {
                let expected_key = match DEPARTURES
                    .iter()
                    .find(|(k, c, _)| k == key && *c == command)
                {
                    Some((_, _, Some(other))) => *other,
                    Some((_, _, None)) => continue,
                    None => key.as_str(),
                };
                if !exists_in_ocs(&command) {
                    continue;
                }
                let expected_files = &by_key[expected_key][4];
                let expected = artwork(
                    expected_key,
                    if expected_key == key {
                        ocs_files
                    } else {
                        expected_files
                    },
                );
                checked += 1;
                if command_icon(&command) != Some(expected.as_slice()) {
                    failures.push(format!("{command} (design icon {expected_key})"));
                }
            }
        }
        assert!(
            failures.is_empty(),
            "commands without their new icon: {failures:#?}"
        );
        assert!(checked > 150, "only {checked} commands checked");
    }

    #[test]
    fn every_command_line_icon_names_a_command_ocs_has() {
        let missing: Vec<&str> = COMMAND_LINE_ICONS
            .iter()
            .map(|&(line, _)| line)
            .filter(|line| !exists_in_ocs(line))
            .collect();
        assert!(missing.is_empty(), "no such command in OCS: {missing:?}");
        let mut seen = std::collections::HashSet::new();
        for (line, _) in COMMAND_LINE_ICONS {
            assert!(seen.insert(*line), "{line} listed twice");
            assert_eq!(*line, line.to_ascii_uppercase(), "{line} is not upper case");
        }
    }

    #[test]
    fn menu_option_rows_take_their_own_icon() {
        for (line, icon) in [
            ("ZOOM P", Z_PREV),
            ("ZOOM W", ZOOM_WIN),
            ("VSCURRENT H", HIDDEN),
            ("-VIEW NEISO", ISO_NE),
            ("DRAWORDER B", DO_BACK),
        ] {
            assert_eq!(command_icon(line), Some(icon), "{line}");
        }
        // The verb takes its default option's icon.
        assert_eq!(command_icon("ZOOM"), command_icon("ZOOM E"));
    }
}
