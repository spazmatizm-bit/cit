use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, PartialEq)]
pub enum InitSystem {
    Systemd,
    Runit,
    Openrc,
    S6,
    Dinit,
    SysV,
}

impl InitSystem {
    pub fn detect() -> Self {
        // Определяем текущий init
        if Path::new("/run/systemd/system").exists() {
            return InitSystem::Systemd;
        }
        if Path::new("/etc/runit").exists() {
            return InitSystem::Runit;
        }
        if Path::new("/etc/init.d/openrc").exists() {
            return InitSystem::Openrc;
        }
        if Path::new("/etc/s6").exists() {
            return InitSystem::S6;
        }
        if Path::new("/etc/dinit.d").exists() {
            return InitSystem::Dinit;
        }
        InitSystem::SysV
    }

    pub fn service_dir(&self) -> &'static str {
        match self {
            InitSystem::Systemd => "/etc/systemd/system",
            InitSystem::Runit => "/etc/runit/runsvdir/default",
            InitSystem::Openrc => "/etc/init.d",
            InitSystem::S6 => "/etc/s6/sv",
            InitSystem::Dinit => "/etc/dinit.d",
            InitSystem::SysV => "/etc/init.d",
        }
    }

    pub fn enable_service(&self, service: &str) -> Result<(), String> {
        match self {
            InitSystem::Systemd => {
                Command::new("systemctl")
                    .args(&["enable", service])
                    .output()
                    .map_err(|e| e.to_string())?;
                Ok(())
            }
            InitSystem::Runit => {
                let src = format!("/etc/sv/{}", service);
                let dst = format!("/etc/runit/runsvdir/default/{}", service);
                fs::remove_file(&dst).ok();
                std::os::unix::fs::symlink(&src, &dst)
                    .map_err(|e| e.to_string())?;
                Ok(())
            }
            InitSystem::Openrc => {
                Command::new("rc-update")
                    .args(&["add", service, "default"])
                    .output()
                    .map_err(|e| e.to_string())?;
                Ok(())
            }
            _ => {
                println!("  ⚠ Service management not implemented for {:?}", self);
                Ok(())
            }
        }
    }

    pub fn disable_service(&self, service: &str) -> Result<(), String> {
        match self {
            InitSystem::Systemd => {
                Command::new("systemctl")
                    .args(&["disable", service])
                    .output()
                    .map_err(|e| e.to_string())?;
                Ok(())
            }
            InitSystem::Runit => {
                let dst = format!("/etc/runit/runsvdir/default/{}", service);
                fs::remove_file(&dst).map_err(|e| e.to_string())?;
                Ok(())
            }
            InitSystem::Openrc => {
                Command::new("rc-update")
                    .args(&["del", service])
                    .output()
                    .map_err(|e| e.to_string())?;
                Ok(())
            }
            _ => Ok(()),
        }
    }
}
