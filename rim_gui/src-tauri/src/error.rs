pub type Result<T> = core::result::Result<T, InstallerError>;

macro_rules! installer_error {
    ($($variant:ident ( $error_ty:ty )),+) => {
        pub enum InstallerError {
            $(
                $variant($error_ty),
            )*
        }
        impl std::fmt::Debug for InstallerError {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $(
                        Self::$variant(e) => write!(f, "{e:?}"),
                    )*
                }
            }
        }
        impl std::fmt::Display for InstallerError {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $(
                        Self::$variant(e) => write!(f, "{e}"),
                    )*
                }
            }
        }
        $(
            impl From<$error_ty> for InstallerError {
                fn from(value: $error_ty) -> Self {
                    Self::$variant(value)
                }
            }
        )*
    };
}

installer_error! {
    Anyhow(anyhow::Error),
    Tauri(tauri::Error),
    Url(url::ParseError)
}

impl std::error::Error for InstallerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            Self::Anyhow(ref e) => Some(e.root_cause()),
            Self::Tauri(ref e) => Some(e),
            Self::Url(ref e) => Some(e),
        }
    }
}

impl From<InstallerError> for tauri::InvokeError {
    fn from(value: InstallerError) -> Self {
        let anyhow_error: anyhow::Error = value.into();
        tauri::InvokeError::from_anyhow(anyhow_error)
    }
}
