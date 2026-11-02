//! Where a host fetches Cisco's OpenH264 library, and how it checks it.
//!
//! limn-video never downloads anything. The host downloads the release for
//! its platform from Cisco onto the person's machine (Cisco's license
//! requires that it come from Cisco, not with the application), decompresses
//! it, checks its SHA-256 against [`CiscoRelease::sha256`], stores it as
//! [`CiscoRelease::file_name`] and passes that path to
//! [`Codec::load`](crate::Codec::load).

/// The Cisco release the `openh264` crate's bindings load. A library of
/// another release is refused by [`Codec::load`](crate::Codec::load).
pub const VERSION: &str = "2.6.0";

/// Where Cisco publishes its binaries.
pub const BASE_URL: &str = "https://ciscobinary.openh264.org/";

/// The text Cisco's license requires where the person turns the codec on
/// or off, and with the license wherever the application presents licenses.
pub const NOTICE: &str = "OpenH264 Video Codec provided by Cisco Systems, Inc.";

/// Cisco's binary license, whose text the application reproduces with its
/// other licenses.
pub const LICENSE_URL: &str = "https://www.openh264.org/BINARY_LICENSE.txt";

/// One platform's library in Cisco's release.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CiscoRelease {
    /// The bzip2-compressed library on Cisco's server.
    pub url: &'static str,
    /// The decompressed library's name, as Cisco names it.
    pub file_name: &'static str,
    /// The SHA-256 of the decompressed library, in lowercase hex.
    pub sha256: &'static str,
}

macro_rules! release {
    ($file:literal, $sha256:literal) => {
        CiscoRelease {
            url: concat!("https://ciscobinary.openh264.org/", $file, ".bz2"),
            file_name: $file,
            sha256: $sha256,
        }
    };
}

// The SHA-256 of each library as decompressed from Cisco's download. Cisco
// publishes an MD5 beside each (`<file>.signed.md5.txt`), which these files
// match, and the `openh264` crate lists the same SHA-256s as the ones it loads.
const RELEASES: [(&str, &str, CiscoRelease); 5] = [
    ("linux", "x86_64", release!("libopenh264-2.6.0-linux64.8.so", "2f0cde7c6a6abcf5cae76942894ea42897fa677bce4ed6c91a24dd1b041d5f04")),
    ("linux", "aarch64", release!("libopenh264-2.6.0-linux-arm64.8.so", "12e7b33623667cdab0e575170c147b1b36eadb77d0d2aa7ceb5afd3e58902140")),
    ("windows", "x86_64", release!("openh264-2.6.0-win64.dll", "2076cb5675ec6c1a4c70e7a2a322552f547b6eeed649d6dfcd9e02a543b24691")),
    ("macos", "x86_64", release!("libopenh264-2.6.0-mac-x64.dylib", "e3dc8bc01fe69363f61fd3c02fd27798537a585eadd38cd808f303d1ee505a19")),
    ("macos", "aarch64", release!("libopenh264-2.6.0-mac-arm64.dylib", "052e98bfcf7a9167d22f3bbb3f5988ef79065591f36af8b52924b22b13624551")),
];

/// The release for an operating system and an architecture, named as
/// [`std::env::consts::OS`] and [`std::env::consts::ARCH`] name them:
/// Linux, Windows and macOS on x86-64 and on 64-bit Arm, except Windows on
/// Arm. `None` elsewhere.
pub fn release_for(os: &str, arch: &str) -> Option<CiscoRelease> {
    RELEASES.iter().find(|(o, a, _)| *o == os && *a == arch).map(|(_, _, release)| *release)
}

/// The release for the platform this is built for.
pub fn current() -> Option<CiscoRelease> {
    release_for(std::env::consts::OS, std::env::consts::ARCH)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_release_is_a_versioned_file_on_ciscos_server_with_a_sha256() {
        for (os, arch, release) in RELEASES {
            assert_eq!(release_for(os, arch), Some(release));
            assert!(release.file_name.contains(&format!("openh264-{VERSION}-")), "{release:?}");
            assert_eq!(release.url, format!("{BASE_URL}{}.bz2", release.file_name));
            assert!(release.sha256.len() == 64 && release.sha256.bytes().all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f')));
        }
        assert_eq!(release_for("windows", "aarch64"), None);
        assert_eq!(release_for("freebsd", "x86_64"), None);
        assert!(current().is_some(), "the platforms limn is built on have a release");
    }
}
