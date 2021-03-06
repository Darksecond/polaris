use core::ops::Deref;
use diesel::prelude::*;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

use crate::db::mount_points;
use crate::db::{ConnectionSource, DB};
use crate::errors::*;

pub trait VFSSource {
	fn get_vfs(&self) -> Result<VFS>;
}

impl VFSSource for DB {
	fn get_vfs(&self) -> Result<VFS> {
		use self::mount_points::dsl::*;
		let mut vfs = VFS::new();
		let connection = self.get_connection();
		let points: Vec<MountPoint> = mount_points
			.select((source, name))
			.get_results(connection.deref())?;
		for point in points {
			vfs.mount(&Path::new(&point.source), &point.name)?;
		}
		Ok(vfs)
	}
}

#[derive(Clone, Debug, Deserialize, Insertable, PartialEq, Queryable, Serialize)]
#[table_name = "mount_points"]
pub struct MountPoint {
	pub source: String,
	pub name: String,
}

pub struct VFS {
	mount_points: HashMap<String, PathBuf>,
}

impl VFS {
	pub fn new() -> VFS {
		VFS {
			mount_points: HashMap::new(),
		}
	}

	pub fn mount(&mut self, real_path: &Path, name: &str) -> Result<()> {
		self.mount_points
			.insert(name.to_owned(), real_path.to_path_buf());
		Ok(())
	}

	pub fn real_to_virtual<P: AsRef<Path>>(&self, real_path: P) -> Result<PathBuf> {
		for (name, target) in &self.mount_points {
			if let Ok(p) = real_path.as_ref().strip_prefix(target) {
				let mount_path = Path::new(&name);
				return if p.components().count() == 0 {
					Ok(mount_path.to_path_buf())
				} else {
					Ok(mount_path.join(p))
				};
			}
		}
		bail!("Real path has no match in VFS")
	}

	pub fn virtual_to_real<P: AsRef<Path>>(&self, virtual_path: P) -> Result<PathBuf> {
		for (name, target) in &self.mount_points {
			let mount_path = Path::new(&name);
			if let Ok(p) = virtual_path.as_ref().strip_prefix(mount_path) {
				return if p.components().count() == 0 {
					Ok(target.clone())
				} else {
					Ok(target.join(p))
				};
			}
		}
		bail!("Virtual path has no match in VFS")
	}

	pub fn get_mount_points(&self) -> &HashMap<String, PathBuf> {
		&self.mount_points
	}
}

#[test]
fn test_virtual_to_real() {
	let mut vfs = VFS::new();
	vfs.mount(Path::new("test_dir"), "root").unwrap();

	let mut correct_path = PathBuf::new();
	correct_path.push("test_dir");
	correct_path.push("somewhere");
	correct_path.push("something.png");

	let mut virtual_path = PathBuf::new();
	virtual_path.push("root");
	virtual_path.push("somewhere");
	virtual_path.push("something.png");

	let found_path = vfs.virtual_to_real(virtual_path.as_path()).unwrap();
	assert!(found_path.to_str() == correct_path.to_str());
}

#[test]
fn test_virtual_to_real_no_trail() {
	let mut vfs = VFS::new();
	vfs.mount(Path::new("test_dir"), "root").unwrap();
	let correct_path = Path::new("test_dir");
	let found_path = vfs.virtual_to_real(Path::new("root")).unwrap();
	assert!(found_path.to_str() == correct_path.to_str());
}

#[test]
fn test_real_to_virtual() {
	let mut vfs = VFS::new();
	vfs.mount(Path::new("test_dir"), "root").unwrap();

	let mut correct_path = PathBuf::new();
	correct_path.push("root");
	correct_path.push("somewhere");
	correct_path.push("something.png");

	let mut real_path = PathBuf::new();
	real_path.push("test_dir");
	real_path.push("somewhere");
	real_path.push("something.png");

	let found_path = vfs.real_to_virtual(real_path.as_path()).unwrap();
	assert!(found_path == correct_path);
}
