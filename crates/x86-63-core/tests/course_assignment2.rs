use x86_63_core::{Command, MachineStatus, Session, SourceModule, StepEvent};

fn assignment_session() -> Session {
    let lesson = x86_63_course::lesson("sum5").unwrap();
    let mut modules = vec![SourceModule::new(lesson.module_name, lesson.source)];
    modules.extend(
        x86_63_course::support_modules(lesson.id)
            .iter()
            .map(|module| SourceModule::new(module.module_name, module.source)),
    );
    Session::from_modules(modules).unwrap()
}

#[test]
fn sum5_accepts_five_separate_lines_and_meets_the_assignment_contract() {
    let mut session = assignment_session();
    let mut calls = Vec::new();
    for input in ["10", "20", "30", "40", "50"] {
        let waiting = session.execute(Command::Continue { max_steps: 2_000 });
        calls.extend(waiting.events.into_iter().filter_map(|event| match event {
            StepEvent::Call {
                target,
                aligned_before,
                ..
            } => Some((target, aligned_before)),
            _ => None,
        }));
        assert!(matches!(
            waiting.view.status,
            MachineStatus::WaitingForInput { .. }
        ));
        session.execute(Command::SubmitInput {
            text: input.to_string(),
        });
    }

    let completed = session.execute(Command::Continue { max_steps: 2_000 });
    calls.extend(completed.events.iter().filter_map(|event| match event {
        StepEvent::Call {
            target,
            aligned_before,
            ..
        } => Some((target.clone(), *aligned_before)),
        _ => None,
    }));
    assert!(matches!(
        completed.view.status,
        MachineStatus::Exited {
            shell_status: 0,
            ..
        }
    ));
    assert_eq!(
        completed.view.io.stdout_escaped,
        "Enter a number: Enter a number: Enter a number: Enter a number: Enter a number: 150\\n"
    );
    assert_eq!(completed.view.stack.rsp, completed.view.stack.top);
    assert!(completed.view.stack.frames.is_empty());
    assert!(completed.diagnostics.is_empty());

    let nums = completed
        .view
        .memory
        .symbols
        .iter()
        .find(|symbol| symbol.name == "nums")
        .unwrap();
    assert_eq!(nums.size, 40);
    let values = completed.view.memory.bytes[nums.offset..nums.offset + nums.size]
        .chunks_exact(8)
        .map(|bytes| u64::from_le_bytes(bytes.try_into().unwrap()))
        .collect::<Vec<_>>();
    assert_eq!(values, [10, 20, 30, 40, 50]);

    assert_eq!(
        calls
            .iter()
            .filter(|(target, _)| target == "readInt")
            .count(),
        5
    );
    assert!(calls.iter().any(|(target, _)| target == "sum_array"));
    assert!(calls.iter().any(|(target, _)| target == "writeInt"));
    assert!(calls.iter().all(|(_, aligned)| *aligned));
}

#[test]
fn sum5_workspace_links_both_course_helpers_and_exposes_procedures() {
    let session = assignment_session();
    let program = session.program();
    assert_eq!(
        program
            .modules
            .iter()
            .map(|module| module.name.as_str())
            .collect::<Vec<_>>(),
        ["sum5.s", "readInt.s", "writeInt.s"]
    );
    for label in ["_start", "read_five", "sum_array", "readInt", "writeInt"] {
        assert!(program.labels.contains_key(label), "missing {label}");
    }
}
