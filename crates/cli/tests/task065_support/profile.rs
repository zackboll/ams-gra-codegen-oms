//! Opt-in evidence outside the source tree; never printed to ordinary stdout.
use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::PathBuf,
    sync::Mutex,
    time::Instant,
};

pub struct Profile {
    file: Option<Mutex<File>>,
    artifacts: Option<PathBuf>,
}

impl Profile {
    pub fn from_env() -> Self {
        Self {
            file: std::env::var_os("AMS_GRA_TASK065_PROFILE").map(|path| {
                Mutex::new(
                    OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(path)
                        .expect("open Task 065 profile"),
                )
            }),
            artifacts: std::env::var_os("AMS_GRA_TASK065_ARTIFACTS").map(PathBuf::from),
        }
    }

    pub fn span<'a>(&'a self, release: &'a str, message: &'a str, phase: &'a str) -> Span<'a> {
        Span {
            profile: self,
            release,
            message,
            phase,
            start: Instant::now(),
        }
    }

    pub fn sources(
        &self,
        release: &str,
        message: &str,
        files: &[ams_gra_oms_codegen_core::GeneratedFile],
    ) {
        if let Some(root) = &self.artifacts {
            let directory = root.join(release).join(message);
            std::fs::create_dir_all(&directory).expect("create Task 065 artifact directory");
            for file in files {
                std::fs::write(directory.join(&file.relative_path), &file.contents)
                    .expect("write actual Task 065 generated source");
            }
        }
    }
}

pub struct Span<'a> {
    profile: &'a Profile,
    release: &'a str,
    message: &'a str,
    phase: &'a str,
    start: Instant,
}

impl Drop for Span<'_> {
    fn drop(&mut self) {
        if let Some(file) = &self.profile.file {
            let row = serde_json::json!({
                "release": self.release,
                "message": self.message,
                "phase": self.phase,
                "seconds": self.start.elapsed().as_secs_f64(),
            });
            writeln!(file.lock().expect("Task 065 profile lock"), "{row}")
                .expect("write Task 065 profile");
        }
    }
}
