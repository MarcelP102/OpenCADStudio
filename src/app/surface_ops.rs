// ADDSURFACE, CLIPSURFACE and INTERSECTSURFACE: applying a surface Boolean
// plan to the drawing as one undo step.

use document::{EntityType, Handle};
use iced::Task;

use super::Message;
use crate::modules::draw::modify::surface_boolean::{
    self as sb, EditPlan, Op, Out, ResultRef, NO_COMMON_SURFACE, SELECT_MORE_PROMPT, SELECT_PROMPT,
};

/// The undo label and summary prefix of `op`.
fn label(op: Op) -> &'static str {
    match op {
        Op::Add => "Add Surface",
        Op::Clip => "Clip Surface",
        Op::Intersect => "Intersect Surface",
    }
}

fn apply_command(op: Op) -> &'static str {
    match op {
        Op::Add => "ADDSURFACEAPPLY",
        Op::Clip => "CLIPSURFACEAPPLY",
        Op::Intersect => "INTERSECTSURFACEAPPLY",
    }
}

fn same_kind(a: &EntityType, b: &EntityType) -> bool {
    std::mem::discriminant(a) == std::mem::discriminant(b)
}

impl super::OpenCADStudio {
    /// Start `op`: with two or more objects selected it applies at once,
    /// otherwise it gathers the selection and applies on Enter.
    pub(super) fn surface_boolean_start(&mut self, op: Op) -> Task<Message> {
        let i = self.active_tab;
        let selected = self.surface_boolean_selection(i);
        if selected.len() >= 2 {
            return self.surface_boolean_apply(op);
        }
        self.surface_boolean_gather(op, selected.len());
        Task::none()
    }

    fn surface_boolean_selection(&self, i: usize) -> Vec<Handle> {
        self.tabs[i].scene.selected_handles_in_order()
    }

    fn surface_boolean_gather(&mut self, op: Op, count: usize) {
        use crate::command::CadCommand;
        use crate::modules::draw::select::SelectObjectsCommand;
        let i = self.active_tab;
        let prompt = if count == 1 {
            SELECT_MORE_PROMPT
        } else {
            SELECT_PROMPT
        };
        let selection = SelectObjectsCommand::with_prompt(op.command(), apply_command(op), prompt);
        self.command_line.push_info(&selection.prompt());
        self.tabs[i].active_cmd = Some(Box::new(selection));
    }

    /// Enter at the selection prompt: plan, then write the plan as one undo
    /// step. A plan that cannot be made leaves the drawing as it is.
    pub(super) fn surface_boolean_apply(&mut self, op: Op) -> Task<Message> {
        let i = self.active_tab;
        let handles = self.surface_boolean_selection(i);
        match handles.len() {
            0 => return Task::none(),
            1 => {
                self.command_line
                    .push_info(crate::t!(SELECT_MORE_PROMPT).as_ref());
                self.surface_boolean_gather(op, 1);
                return Task::none();
            }
            _ => {}
        }
        let plan = match sb::plan(op, &self.tabs[i].scene.document, &handles) {
            Ok(plan) => plan,
            Err(message) => {
                let message = if message == crate::t!(NO_COMMON_SURFACE) {
                    message
                } else {
                    format!("{}: {message}", op.command())
                };
                self.command_line.push_error(&message);
                return Task::none();
            }
        };
        if plan.is_empty() {
            self.command_line.push_info(&plan.summary);
            return Task::none();
        }
        self.push_undo_snapshot(i, label(op));
        let mut wrote = false;
        match self.write_surface_plan(i, &plan, &mut wrote) {
            Ok(results) => {
                let scene = &mut self.tabs[i].scene;
                scene.deselect_all();
                for handle in &results {
                    scene.select_entity(*handle, false);
                }
                self.tabs[i].dirty = true;
                self.refresh_properties();
                self.command_line.push_output(&plan.summary);
            }
            Err(error) => {
                // Atomic: take back whatever was written. With nothing
                // written the step is dropped, so the previous command stays.
                if wrote {
                    self.undo_steps_reported(1, false);
                    // The partial step must not come back with REDO.
                    self.clear_redo_history(i);
                } else {
                    self.discard_last_undo_entry(i);
                }
                self.command_line
                    .push_error(&format!("{}: {error}", op.command()));
            }
        }
        Task::none()
    }

    /// Add one result; a REGION gets its exact body.
    fn add_surface_result(&mut self, i: usize, out: Out) -> Result<Handle, String> {
        let handle = match out {
            Out::Entity(entity) => self.tabs[i].scene.add_entity(entity),
            Out::Region { region, body } => self.add_region_model_preserving_style(region, body),
        };
        if handle.is_null() {
            Err(crate::t!("a result object could not be added").into_owned())
        } else {
            Ok(handle)
        }
    }

    /// Write `plan`; returns the result objects, to select. `wrote` turns
    /// true at the first change.
    fn write_surface_plan(
        &mut self,
        i: usize,
        plan: &EditPlan,
        wrote: &mut bool,
    ) -> Result<Vec<Handle>, String> {
        let mut select: Vec<Handle> = Vec::new();
        let mut remove: Vec<Handle> = plan.removals.clone();
        // The handle each result ended up with, for the hatches below.
        let mut rep_handles: Vec<Vec<Handle>> = Vec::with_capacity(plan.replacements.len());
        for rep in &plan.replacements {
            let old = self.tabs[i]
                .scene
                .document
                .get_entity(rep.handle)
                .cloned()
                .ok_or_else(|| crate::t!("a selected object no longer exists").into_owned())?;
            let mut handles = Vec::with_capacity(rep.results.len());
            let mut above = rep.handle;
            for (k, out) in rep.results.iter().cloned().enumerate() {
                let in_place = match &out {
                    Out::Entity(e) => k == 0 && same_kind(e, &old),
                    Out::Region { .. } => false,
                };
                let handle = if in_place {
                    // The main result takes over the source object: same
                    // handle, so its drawing-order slot, extension dictionary
                    // and reactors stay.
                    let Out::Entity(mut entity) = out else {
                        unreachable!()
                    };
                    let c = entity.common_mut();
                    c.handle = rep.handle;
                    c.xdictionary_handle = old.common().xdictionary_handle;
                    c.reactors = old.common().reactors.clone();
                    if !self.tabs[i].scene.update_entity(entity) {
                        return Err(
                            crate::t!("a selected object could not be changed").into_owned()
                        );
                    }
                    rep.handle
                } else {
                    // Another kind, or one more result: a new object right
                    // above the previous one.
                    let handle = self.add_surface_result(i, out)?;
                    self.apply_draw_order(i, &[handle], None, Some((true, &[above])));
                    if k == 0 {
                        remove.push(rep.handle);
                    }
                    handle
                };
                *wrote = true;
                above = handle;
                handles.push(handle);
            }
            select.extend(&handles);
            rep_handles.push(handles);
        }
        // New hatches go in before the outlines they follow, so they sit
        // just below them; their association is set once the outlines exist.
        let mut hatch_handles: Vec<Handle> = Vec::with_capacity(plan.hatches.len());
        for update in &plan.hatches {
            let existing = update.hatch.common.handle;
            if existing.is_null() {
                let mut h = (*update.hatch).clone();
                h.paths = update.bound.iter().map(|(p, _)| p.clone()).collect();
                h.is_associative = false;
                hatch_handles
                    .push(self.add_surface_result(i, Out::Entity(EntityType::Hatch(Box::new(h))))?);
                *wrote = true;
            } else {
                hatch_handles.push(existing);
            }
        }
        let mut add_handles: Vec<Handle> = Vec::with_capacity(plan.additions.len());
        for out in &plan.additions {
            add_handles.push(self.add_surface_result(i, out.clone())?);
            *wrote = true;
        }
        select.extend(&add_handles);
        // Associate every hatch with the results it follows.
        for (update, handle) in plan.hatches.iter().zip(&hatch_handles) {
            let mut h = (*update.hatch).clone();
            h.common.handle = *handle;
            if let Some(EntityType::Hatch(live)) = self.tabs[i].scene.document.get_entity(*handle) {
                h.common.xdictionary_handle = live.common.xdictionary_handle;
                h.common.reactors = live.common.reactors.clone();
            }
            h.paths = update
                .bound
                .iter()
                .map(|(path, r)| {
                    let mut path = path.clone();
                    path.boundary_handles = vec![match *r {
                        ResultRef::Replacement { rep, index } => rep_handles[rep][index],
                        ResultRef::Addition(n) => add_handles[n],
                    }];
                    path
                })
                .chain(update.kept.iter().cloned())
                .collect();
            h.is_associative = h.paths.iter().any(|p| !p.boundary_handles.is_empty());
            if !self.tabs[i]
                .scene
                .update_entity(EntityType::Hatch(Box::new(h)))
            {
                return Err(crate::t!("a hatch could not be changed").into_owned());
            }
            *wrote = true;
        }
        if !remove.is_empty() {
            self.tabs[i].scene.erase_entities_exact(&remove);
            *wrote = true;
        }
        Ok(select)
    }
}

#[cfg(test)]
mod tests {
    use crate::app::OpenCADStudio;
    use document::entities::{
        BoundaryEdge, BoundaryPath, Circle, Hatch, LwPolyline, LwVertex, PolylineEdge,
    };
    use document::objects::ObjectType;
    use document::types::{Vector2, Vector3};
    use document::{EntityType, Handle};

    fn app() -> OpenCADStudio {
        let mut app = OpenCADStudio::new_for_test();
        app.automation_op(r#"{"op":"new"}"#);
        app
    }

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> EntityType {
        let mut p = LwPolyline::new();
        for (x, y) in [(x0, y0), (x1, y0), (x1, y1), (x0, y1)] {
            p.vertices.push(LwVertex::new(Vector2::new(x, y)));
        }
        p.is_closed = true;
        EntityType::LwPolyline(p)
    }

    fn circle(x: f64, y: f64, r: f64) -> EntityType {
        let mut c = Circle::new();
        c.center = Vector3::new(x, y, 0.0);
        c.radius = r;
        EntityType::Circle(c)
    }

    fn add(app: &mut OpenCADStudio, e: EntityType) -> Handle {
        let i = app.active_tab;
        app.tabs[i].scene.add_entity(e)
    }

    fn select(app: &mut OpenCADStudio, handles: &[Handle]) {
        let i = app.active_tab;
        app.tabs[i].scene.deselect_all();
        for h in handles {
            app.tabs[i].scene.select_entity(*h, false);
        }
    }

    fn run(app: &mut OpenCADStudio, cmd: &str) {
        let line = serde_json::json!({ "op": "run", "cmd": cmd }).to_string();
        app.automation_op(&line);
    }

    fn handles(app: &OpenCADStudio) -> Vec<Handle> {
        let i = app.active_tab;
        let mut v: Vec<Handle> = app.tabs[i]
            .scene
            .document
            .entities()
            .map(|e| e.common().handle)
            .collect();
        v.sort();
        v
    }

    fn entity(app: &OpenCADStudio, h: Handle) -> EntityType {
        app.tabs[app.active_tab]
            .scene
            .document
            .get_entity(h)
            .cloned()
            .expect("entity")
    }

    /// Effective drawing-order key: a sort-table entry, else the handle.
    fn draw_key(app: &OpenCADStudio, h: Handle) -> u64 {
        let doc = &app.tabs[app.active_tab].scene.document;
        doc.objects
            .values()
            .filter_map(|o| match o {
                ObjectType::SortEntitiesTable(t) => Some(t),
                _ => None,
            })
            .flat_map(|t| {
                t.entries()
                    .map(|e| (e.entity_handle, e.sort_handle))
                    .collect::<Vec<_>>()
            })
            .find(|(e, _)| *e == h)
            .map_or(h.value(), |(_, s)| s.value())
    }

    #[test]
    fn clip_by_alias_is_one_undo_step_and_selects_the_results() {
        let mut app = app();
        let base = add(&mut app, rect(0., 0., 10., 10.));
        let hole = add(&mut app, circle(5., 5., 2.));
        let before = handles(&app);
        select(&mut app, &[hole, base]);
        run(&mut app, "CS");
        let after = handles(&app);
        assert_eq!(after.len(), 3, "outline, hole outline and the circle");
        let new: Vec<Handle> = after
            .iter()
            .copied()
            .filter(|h| !before.contains(h))
            .collect();
        assert_eq!(new.len(), 1);
        // The source keeps its handle; the hole is a new two-bulge outline
        // drawn just above it, still under the circle that cut it.
        assert!(matches!(entity(&app, base), EntityType::LwPolyline(p) if p.vertices.len() == 4));
        assert!(matches!(entity(&app, new[0]), EntityType::LwPolyline(p) if p.vertices.len() == 2));
        assert!(
            draw_key(&app, base) < draw_key(&app, new[0])
                && draw_key(&app, new[0]) < draw_key(&app, hole)
        );
        let i = app.active_tab;
        let mut selected: Vec<Handle> = app.tabs[i].scene.selected.iter().copied().collect();
        selected.sort();
        let mut want = vec![base, new[0]];
        want.sort();
        assert_eq!(selected, want, "only the results are selected");

        app.undo_active_tab();
        assert_eq!(handles(&app), before, "one undo restores the drawing");
        assert!(matches!(entity(&app, base), EntityType::LwPolyline(p) if p.vertices.len() == 4));
    }

    #[test]
    fn intersect_gives_the_piece_its_own_associative_hatch() {
        let mut app = app();
        let a = add(&mut app, rect(0., 0., 4., 4.));
        let mut hatch = Hatch::solid();
        let mut path = BoundaryPath::new();
        let mut pe = PolylineEdge::new(Vec::new(), true);
        for (x, y) in [(0., 0.), (4., 0.), (4., 4.), (0., 4.)] {
            pe.add_vertex(Vector2::new(x, y), 0.0);
        }
        path.edges.push(BoundaryEdge::Polyline(pe));
        path.boundary_handles = vec![a];
        hatch.paths.push(path);
        hatch.is_associative = true;
        let h = add(&mut app, EntityType::Hatch(Box::new(hatch)));
        let b = add(&mut app, rect(2., 2., 6., 6.));
        let before = handles(&app);
        select(&mut app, &[b, a]);
        run(&mut app, "INTERSECTSURFACE");
        let new: Vec<Handle> = handles(&app)
            .into_iter()
            .filter(|x| !before.contains(x))
            .collect();
        assert_eq!(new.len(), 2, "the piece and its hatch: {new:?}");
        let (piece, piece_hatch) = match (entity(&app, new[0]), entity(&app, new[1])) {
            (EntityType::Hatch(_), _) => (new[1], new[0]),
            _ => (new[0], new[1]),
        };
        let EntityType::Hatch(ph) = entity(&app, piece_hatch) else {
            panic!("expected a hatch")
        };
        assert!(ph.is_associative);
        assert_eq!(ph.paths[0].boundary_handles, vec![piece]);
        assert!(
            draw_key(&app, piece_hatch) < draw_key(&app, piece),
            "the hatch sits below its outline"
        );
        let EntityType::Hatch(orig) = entity(&app, h) else {
            panic!()
        };
        assert_eq!(
            orig.paths[0].boundary_handles,
            vec![a],
            "the source hatch is untouched"
        );
        app.undo_active_tab();
        assert_eq!(handles(&app), before);
    }

    #[test]
    fn one_shape_keeps_the_command_asking_for_more() {
        let mut app = app();
        let a = add(&mut app, rect(0., 0., 4., 4.));
        add(&mut app, rect(2., 2., 6., 6.));
        let before = handles(&app);
        select(&mut app, &[a]);
        run(&mut app, "ADDSURFACE");
        let i = app.active_tab;
        assert!(
            app.tabs[i].active_cmd.is_some(),
            "still gathering the selection"
        );
        assert_eq!(handles(&app), before, "nothing changed");
    }
}
