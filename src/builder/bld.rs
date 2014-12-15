use std::io::ALL_PERMISSIONS;
use std::io::fs::{mkdir_recursive};
use std::io::fs::PathExtensions;

use config::builders as B;

use super::context::BuildContext;
use super::commands::debian;
use super::commands::generic;
use super::download::download_file;
use super::tarcmd::unpack_file;
use container::util::clean_dir;
use container::mount::{bind_mount, unmount};


pub trait BuildCommand {
    fn build(&self, ctx: &mut BuildContext) -> Result<(), String>;
}


impl BuildCommand for B::Builder {
    fn build(&self, ctx: &mut BuildContext) -> Result<(), String> {
        match self {
            &B::UbuntuCore(ref name) => {
                debian::fetch_ubuntu_core(ctx, name)
            }
            &B::Sh(ref text) => {
                generic::run_command(ctx,
                    &["/bin/sh".to_string(),
                      "-c".to_string(),
                      text.to_string()])
            }
            &B::Cmd(ref cmd) => {
                generic::run_command(ctx, cmd.as_slice())
            }
            &B::Env(ref pairs) => {
                for (k, v) in pairs.iter() {
                    ctx.environ.insert(k.clone(), v.clone());
                }
                Ok(())
            }
            &B::Remove(ref path) => {
                try!(clean_dir(path, true));
                ctx.add_remove_dir(path.clone());
                Ok(())
            }
            &B::EmptyDir(ref path) => {
                try!(clean_dir(path, false));
                ctx.add_empty_dir(path.clone());
                Ok(())
            }
            &B::EnsureDir(ref path) => {
                let fpath = path.path_relative_from(&Path::new("/")).unwrap();
                try!(mkdir_recursive(
                    &Path::new("/vagga/root").join(fpath), ALL_PERMISSIONS)
                    .map_err(|e| format!("Error creating dir: {}", e)));
                ctx.add_ensure_dir(path.clone());
                Ok(())
            }
            &B::Depends(_) => {
                Ok(())
            }
            &B::Tar(ref tar) => {
                let fpath = Path::new("/vagga/root").join(
                    tar.path.path_relative_from(&Path::new("/")).unwrap());
                let filename = try!(download_file(ctx, &tar.url));
                // TODO(tailhook) check sha256 sum
                if tar.subdir == Path::new(".") {
                    try!(unpack_file(ctx, &filename, &fpath, &[], &[]));
                } else {
                    let tmppath = Path::new("/vagga/root/tmp")
                        .join(filename.filename_str().unwrap());
                    let tmpsub = tmppath.join(&tar.subdir);
                    try!(mkdir_recursive(&tmpsub, ALL_PERMISSIONS)
                         .map_err(|e| format!("Error making dir: {}", e)));
                    if !fpath.exists() {
                        try!(mkdir_recursive(&fpath, ALL_PERMISSIONS)
                             .map_err(|e| format!("Error making dir: {}", e)));
                    }
                    try!(bind_mount(&fpath, &tmpsub));
                    let res = unpack_file(ctx, &filename, &tmppath,
                        &[tar.subdir.clone()], &[]);
                    try!(unmount(&tmpsub));
                    try!(res);
                }
                Ok(())
            }
        }
    }
}
