//! Custom entry types for config maps (build arguments, volumes, environment variables).
//!
//! Each entry type is a tri-state enum that supports a literal value, an inherit/same-path
//! shorthand, and a removal sentinel. Custom `Serialize`/`Deserialize` implementations encode
//! values as strings and shorthands/sentinels as booleans so that TOML, environment variables,
//! and CLI sources all use the same representation.

use serde::de::{Error as DeError, Visitor};
use serde::{Deserialize, Serialize};
use std::fmt::{Formatter, Result as FmtResult};

/// A build argument entry: a literal value, a host-inherited value, or a removal sentinel.
///
/// In TOML and environment variables: a string = literal value; `true` = inherit from host
/// environment; `false` = remove.
///
/// On the CLI: `"KEY=value"` = literal value; `"KEY"` (no `=`) = inherit from host environment;
/// `"!KEY"` = remove.
#[derive(Debug, Clone)]
pub(crate) enum BuildArgumentEntry {
    /// A literal value to pass as a `--build-arg` to `docker build`.
    Value(String),
    /// Inherit the build argument value from the host environment.
    Inherit,
    /// Removal sentinel.
    Remove,
}

impl Serialize for BuildArgumentEntry {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match *self {
            Self::Value(ref value) => serializer.serialize_str(value),
            Self::Inherit => serializer.serialize_bool(true),
            Self::Remove => serializer.serialize_bool(false),
        }
    }
}

impl<'de> Deserialize<'de> for BuildArgumentEntry {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct BuildArgumentEntryVisitor;

        impl Visitor<'_> for BuildArgumentEntryVisitor {
            type Value = BuildArgumentEntry;

            fn expecting(&self, formatter: &mut Formatter<'_>) -> FmtResult {
                formatter
                    .write_str("a string value, `true` to inherit from host, or `false` to remove")
            }

            fn visit_str<E: DeError>(self, v: &str) -> Result<Self::Value, E> {
                Ok(BuildArgumentEntry::Value(String::from(v)))
            }

            fn visit_string<E: DeError>(self, v: String) -> Result<Self::Value, E> {
                Ok(BuildArgumentEntry::Value(v))
            }

            fn visit_bool<E: DeError>(self, v: bool) -> Result<Self::Value, E> {
                if v {
                    Ok(BuildArgumentEntry::Inherit)
                } else {
                    Ok(BuildArgumentEntry::Remove)
                }
            }
        }

        deserializer.deserialize_any(BuildArgumentEntryVisitor)
    }
}

/// A volume entry: an explicit host path, a same-path shorthand, or a removal sentinel.
///
/// In TOML and environment variables: a string = host path; `true` = mount at the same path as
/// the container path key; `false` = remove.
///
/// On the CLI: `"/host:/container"` = explicit mount; `"/path"` (no colon) = same-path shorthand;
/// `"!/container"` = remove.
#[derive(Debug, Clone)]
pub(crate) enum VolumeEntry {
    /// The host path to mount at the container path key.
    Active(String),
    /// Mount the container path key at the same path on the host.
    SamePath,
    /// Removal sentinel.
    Remove,
}

impl Serialize for VolumeEntry {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match *self {
            Self::Active(ref host_path) => serializer.serialize_str(host_path),
            Self::SamePath => serializer.serialize_bool(true),
            Self::Remove => serializer.serialize_bool(false),
        }
    }
}

impl<'de> Deserialize<'de> for VolumeEntry {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct VolumeEntryVisitor;

        impl Visitor<'_> for VolumeEntryVisitor {
            type Value = VolumeEntry;

            fn expecting(&self, formatter: &mut Formatter<'_>) -> FmtResult {
                formatter.write_str(
                    "a host path string, `true` for same-path mount, or `false` to remove",
                )
            }

            fn visit_str<E: DeError>(self, v: &str) -> Result<Self::Value, E> {
                Ok(VolumeEntry::Active(String::from(v)))
            }

            fn visit_string<E: DeError>(self, v: String) -> Result<Self::Value, E> {
                Ok(VolumeEntry::Active(v))
            }

            fn visit_bool<E: DeError>(self, v: bool) -> Result<Self::Value, E> {
                if v {
                    Ok(VolumeEntry::SamePath)
                } else {
                    Ok(VolumeEntry::Remove)
                }
            }
        }

        deserializer.deserialize_any(VolumeEntryVisitor)
    }
}

/// An environment variable entry.
///
/// In TOML and environment variables: a string = literal value; `true` = inherit from host;
/// `false` = remove / suppress.
///
/// On the CLI: `"KEY=value"` = literal value; `"KEY"` (no `=`) = inherit from host; `"!KEY"` =
/// remove.
#[derive(Debug, Clone)]
pub(crate) enum EnvironmentVariableEntry {
    /// A literal value to pass into the container.
    Value(String),
    /// Inherit the variable from the host environment.
    Inherit,
    /// Remove / suppress the variable in the container.
    Remove,
}

impl Serialize for EnvironmentVariableEntry {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match *self {
            Self::Value(ref value) => serializer.serialize_str(value),
            Self::Inherit => serializer.serialize_bool(true),
            Self::Remove => serializer.serialize_bool(false),
        }
    }
}

impl<'de> Deserialize<'de> for EnvironmentVariableEntry {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct EnvironmentVariableEntryVisitor;

        impl Visitor<'_> for EnvironmentVariableEntryVisitor {
            type Value = EnvironmentVariableEntry;

            fn expecting(&self, formatter: &mut Formatter<'_>) -> FmtResult {
                formatter.write_str("a string value, `true` to inherit, or `false` to remove")
            }

            fn visit_str<E: DeError>(self, v: &str) -> Result<Self::Value, E> {
                Ok(EnvironmentVariableEntry::Value(String::from(v)))
            }

            fn visit_string<E: DeError>(self, v: String) -> Result<Self::Value, E> {
                Ok(EnvironmentVariableEntry::Value(v))
            }

            fn visit_bool<E: DeError>(self, v: bool) -> Result<Self::Value, E> {
                if v {
                    Ok(EnvironmentVariableEntry::Inherit)
                } else {
                    Ok(EnvironmentVariableEntry::Remove)
                }
            }
        }

        deserializer.deserialize_any(EnvironmentVariableEntryVisitor)
    }
}
