use sysinfo::{System, ProcessesToUpdate, ProcessRefreshKind, UpdateKind};
fn main() {
    let mut sys = System::new();
    let kind = ProcessRefreshKind::new()
        .with_cpu()
        .with_memory()
        .with_disk_usage()
        .with_user(UpdateKind::OnlyIfNotSet)
        .with_cmd(UpdateKind::OnlyIfNotSet)
        .with_exe(UpdateKind::OnlyIfNotSet)
        .with_environ(UpdateKind::Never);
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, kind);
}
