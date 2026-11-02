use x86_63_core::{Command, MachineStatus, Session, SourceModule, StepEvent};

fn session(id: &str) -> Session {
    let lesson = x86_63_course::lesson(id).unwrap();
    let mut modules = vec![SourceModule::new(lesson.module_name, lesson.source)];
    modules.extend(
        x86_63_course::support_modules(id)
            .iter()
            .map(|module| SourceModule::new(module.module_name, module.source)),
    );
    Session::from_modules(modules).unwrap_or_else(|error| panic!("{id}: {error}"))
}

fn exit_status(result: &x86_63_core::CommandResult) -> u8 {
    match &result.view.status {
        MachineStatus::Exited { shell_status, .. } => *shell_status,
        status => panic!("expected an exit, found {status:?}"),
    }
}

#[test]
fn every_guide_program_matches_its_gnu_exit_status() {
    // Statuses were confirmed with GNU as/ld by scripts/differential-guide.sh.
    for (id, status) in [
        ("guidelocallabels", 0),
        ("guidesign", 255),
        ("guidesum", 55),
        ("guidemax", 11),
        ("guideabs", 42),
        ("guidelocals", 60),
        ("guidefact", 120),
        ("signeddivide", 7),
    ] {
        assert_eq!(x86_63_course::collection(id, 0), "guide");
        let result = session(id).execute(Command::Continue { max_steps: 2_000 });
        assert_eq!(exit_status(&result), status, "{id}");
    }
}

#[test]
fn local_labels_print_both_messages_in_call_order() {
    let result = session("guidelocallabels").execute(Command::Continue { max_steps: 2_000 });
    assert_eq!(
        result.view.io.stdout_escaped,
        "In the second sublabel\\n\\0In the first sublabel\\n\\0"
    );
}

#[test]
fn sum_to_n_reads_with_the_real_helpers_and_branches_with_jz() {
    let mut session = session("guidesumn");
    let waiting = session.execute(Command::Continue { max_steps: 2_000 });
    assert!(matches!(
        waiting.view.status,
        MachineStatus::WaitingForInput { .. }
    ));
    session.execute(Command::SubmitInput {
        text: "10".to_string(),
    });
    let mut saw_jz = false;
    loop {
        let result = session.execute(Command::Step);
        saw_jz |= result.events.iter().any(|event| {
            matches!(event, StepEvent::Branch { condition, taken: true, .. } if condition == "jz")
        });
        if !result.view.status.can_step() {
            assert_eq!(result.view.io.stdout_escaped, "55");
            assert_eq!(exit_status(&result), 0);
            break;
        }
    }
    assert!(saw_jz);
}

#[test]
fn max_of_array_uses_an_equate_expression_for_its_length() {
    let session = session("guidemax");
    assert_eq!(session.program().constants["arrlen"], "5");
}
