use x86_63_core::{Command, MachineStatus, Session, SourceModule};

fn session(id: &str, source: String) -> Session {
    let lesson = x86_63_course::lesson(id).unwrap();
    let mut modules = vec![SourceModule::new(lesson.module_name, source)];
    modules.extend(
        x86_63_course::support_modules(id)
            .iter()
            .map(|module| SourceModule::new(module.module_name, module.source)),
    );
    Session::from_modules(modules).unwrap_or_else(|error| panic!("{id}: {error}"))
}

fn run(id: &str, source: String) -> x86_63_core::CommandResult {
    session(id, source).execute(Command::Continue { max_steps: 2_000 })
}

fn register<'a>(view: &'a x86_63_core::MachineView, name: &str) -> &'a str {
    &view
        .registers
        .iter()
        .find(|register| register.name == name)
        .unwrap()
        .unsigned
}

#[test]
fn syntax_labs_begin_with_the_intended_diagnostics() {
    for (id, code) in [("missingimmediate", "E212"), ("reversedoperands", "E215")] {
        let lesson = x86_63_course::lesson(id).unwrap();
        let error =
            Session::from_modules(vec![SourceModule::new(lesson.module_name, lesson.source)])
                .err()
                .expect("starter should be deliberately invalid");
        assert_eq!(error.diagnostics[0].code, code);
        assert!(x86_63_course::challenge(id).is_some());
    }
}

#[test]
fn every_repair_lab_has_a_course_shaped_solution() {
    let missing = x86_63_course::lesson("missingimmediate").unwrap();
    let result = run(
        missing.id,
        missing.source.replace("mov 60,%rax", "mov $60,%rax"),
    );
    assert!(matches!(
        result.view.status,
        MachineStatus::Exited {
            shell_status: 0,
            ..
        }
    ));

    let reversed = x86_63_course::lesson("reversedoperands").unwrap();
    let result = run(
        reversed.id,
        reversed.source.replace("mov %rax,$60", "mov $60,%rax"),
    );
    assert_eq!(register(&result.view, "rax"), "60");

    let width = x86_63_course::lesson("wrongwidth").unwrap();
    let result = run(
        width.id,
        width
            .source
            .replace("movq score(%rip),%rdi", "movl score(%rip),%edi"),
    );
    assert_eq!(register(&result.view, "rdi"), "255");

    let caller_saved = x86_63_course::lesson("callersavedloop").unwrap();
    let result = run(caller_saved.id, caller_saved.source.replace("%rcx", "%rbx"));
    assert_eq!(result.view.io.stdout_escaped, "321");
    assert!(result.diagnostics.is_empty());

    let alignment = x86_63_course::lesson("misalignedcall").unwrap();
    let repaired = alignment.source.replace(
        "\tcall writeInt",
        "\tsub $8,%rsp\n\tcall writeInt\n\tadd $8,%rsp",
    );
    let result = run(alignment.id, repaired);
    assert_eq!(result.view.io.stdout_escaped, "42");
    assert!(result.diagnostics.is_empty());
}

#[test]
fn misaligned_call_lab_emits_a_nonfatal_abi_warning() {
    let lesson = x86_63_course::lesson("misalignedcall").unwrap();
    let result = run(lesson.id, lesson.source.to_string());
    assert!(matches!(result.view.status, MachineStatus::Exited { .. }));
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(result.diagnostics[0].code, "W401");
    assert_eq!(result.view.io.stdout_escaped, "42");
}
