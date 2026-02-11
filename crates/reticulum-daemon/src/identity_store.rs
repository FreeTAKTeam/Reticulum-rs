use std::fs;
use std::io;
use std::path::Path;

use rand_core::OsRng;

use reticulum::identity::PrivateIdentity;

pub fn load_or_create_identity(path: &Path) -> io::Result<PrivateIdentity> {
    match fs::read(path) {
        Ok(bytes) => {
            return PrivateIdentity::from_private_key_bytes(&bytes).map_err(|err| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("invalid identity: {err:?}"),
                )
            });
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => {}
        Err(err) => return Err(err),
    }

    let identity = PrivateIdentity::new_from_rand(OsRng);
    fs::write(path, identity.to_private_key_bytes())?;
    Ok(identity)
}
