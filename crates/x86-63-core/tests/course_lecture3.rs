use x86_63_core::{Command, MachineStatus, MachineView, Session, SourceModule};

fn register<'a>(view: &'a MachineView, name: &str) -> &'a str {
    &view
        .registers
        .iter()
        .find(|register| register.name == name)
        .expect("canonical register exists")
        .unsigned
}

#[test]
fn maintained_lecture_3_examples_have_the_expected_outcomes() {
    for lesson in x86_63_course::lessons()
        .iter()
        .filter(|lesson| lesson.lecture == 3)
    {
        let mut session =
            Session::from_modules(vec![SourceModule::new(lesson.module_name, lesson.source)])
                .unwrap_or_else(|error| panic!("{} did not assemble: {error}", lesson.id));
        let result = session.execute(Command::Continue { max_steps: 100 });

        match lesson.id {
            "first" => assert!(matches!(
                result.view.status,
                MachineStatus::Faulted { ref code, .. } if code == "fell_off_text"
            )),
            "firstfixed" => assert!(matches!(
                result.view.status,
                MachineStatus::Exited {
                    shell_status: 255,
                    ..
                }
            )),
            "firstadd" => {
                assert_eq!(register(&result.view, "rcx"), "30");
                assert_eq!(register(&result.view, "rdi"), "30");
                assert!(matches!(
                    result.view.status,
                    MachineStatus::Exited {
                        shell_status: 30,
                        ..
                    }
                ));
            }
            "firstsub" => {
                assert_eq!(register(&result.view, "rcx"), "10");
                assert_eq!(register(&result.view, "rdi"), "10");
                assert!(matches!(
                    result.view.status,
                    MachineStatus::Exited {
                        shell_status: 10,
                        ..
                    }
                ));
            }
            unexpected => panic!("add an explicit expectation for {unexpected}"),
        }
        assert!(result.diagnostics.is_empty(), "{}", lesson.id);
    }
}

#[test]
fn reverse_step_crosses_a_syscall_boundary() {
    let lesson = x86_63_course::lesson("firstadd").unwrap();
    let mut session =
        Session::from_modules(vec![SourceModule::new(lesson.module_name, lesson.source)]).unwrap();

    session.execute(Command::Continue { max_steps: 100 });
    let reversed = session.execute(Command::Back);
    assert_eq!(reversed.view.status, MachineStatus::Paused);
    assert_eq!(
        reversed.view.next_text.as_deref().map(str::trim),
        Some("syscall")
    );
    assert_eq!(register(&reversed.view, "rdi"), "30");

    let replayed = session.execute(Command::Step);
    assert!(matches!(
        replayed.view.status,
        MachineStatus::Exited {
            shell_status: 30,
            ..
        }
    ));
}
