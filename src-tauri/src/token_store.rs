//! Windows Credential Manager protects the saved session for the current OS user.
//! The generation check and the vault mutation share a lock: logout cannot race a
//! single-use refresh token rotation and accidentally save the account again.
use serde::{de::DeserializeOwned, Serialize};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Mutex,
};
static STORE: Mutex<()> = Mutex::new(());
const TARGET: &str = "dev.andreazero.backgroundaudio/twitch";

pub fn save<T: Serialize>(value: &T, generation: &AtomicU64, expected: u64) -> Result<(), String> {
    save_for(TARGET, value, generation, expected)
}

pub const GOOGLE: &str = "dev.andreazero.backgroundaudio/google";

pub fn save_for<T: Serialize>(
    target: &str,
    value: &T,
    generation: &AtomicU64,
    expected: u64,
) -> Result<(), String> {
    let _lock = STORE
        .lock()
        .map_err(|_| "Archivio account non disponibile")?;
    if generation.load(Ordering::SeqCst) != expected {
        return Ok(());
    }
    let bytes =
        serde_json::to_vec(value).map_err(|_| "Impossibile preparare la sessione account")?;
    native::write(target, &bytes)
}

pub fn read<T: DeserializeOwned>() -> Result<Option<T>, String> {
    read_for(TARGET)
}

pub fn read_for<T: DeserializeOwned>(target: &str) -> Result<Option<T>, String> {
    let _lock = STORE
        .lock()
        .map_err(|_| "Archivio account non disponibile")?;
    native::read(target)?
        .map(|bytes| {
            serde_json::from_slice(&bytes)
                .map_err(|_| "Account salvato non leggibile. Ricollega il servizio".to_string())
        })
        .transpose()
}

pub fn clear(generation: &AtomicU64, next: u64) -> Result<(), String> {
    clear_for(TARGET, generation, next)
}

pub fn clear_for(target: &str, generation: &AtomicU64, next: u64) -> Result<(), String> {
    let _lock = STORE
        .lock()
        .map_err(|_| "Archivio account non disponibile")?;
    generation.store(next, Ordering::SeqCst);
    native::delete(target)
}

#[cfg(windows)]
mod native {
    use super::*;
    use windows_sys::Win32::{
        Foundation::ERROR_NOT_FOUND,
        Security::Credentials::{
            CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE,
            CRED_TYPE_GENERIC,
        },
    };
    fn target(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(Some(0)).collect()
    }
    pub fn write(name: &str, bytes: &[u8]) -> Result<(), String> {
        if bytes.len() > 2560 {
            return Err("Sessione account troppo grande per Gestione credenziali Windows".into());
        }
        let mut target = target(name);
        let mut data = bytes.to_vec();
        let mut credential: CREDENTIALW = unsafe { std::mem::zeroed() };
        credential.Type = CRED_TYPE_GENERIC;
        credential.TargetName = target.as_mut_ptr();
        credential.CredentialBlobSize = data.len() as u32;
        credential.CredentialBlob = data.as_mut_ptr();
        credential.Persist = CRED_PERSIST_LOCAL_MACHINE;
        // CredWrite copies the blob before returning; no pointer escapes this call.
        let success = unsafe { CredWriteW(&credential, 0) };
        data.fill(0);
        if success == 0 {
            return Err(
                "Windows non riesce a ricordare l'account. Verifica Gestione credenziali".into(),
            );
        }
        Ok(())
    }
    pub fn read(name: &str) -> Result<Option<Vec<u8>>, String> {
        let target = target(name);
        let mut credential = std::ptr::null_mut();
        if unsafe { CredReadW(target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut credential) } == 0 {
            return if std::io::Error::last_os_error().raw_os_error() == Some(ERROR_NOT_FOUND as i32)
            {
                Ok(None)
            } else {
                Err("Windows non riesce a leggere l'account salvato".into())
            };
        }
        // Credential memory is owned by Windows and freed exactly once.
        let bytes = unsafe {
            if (*credential).CredentialBlobSize == 0
                || (*credential).CredentialBlobSize > 2560
                || (*credential).CredentialBlob.is_null()
            {
                CredFree(credential as _);
                return Err("Sessione account salvata non valida. Ricollega l'account".into());
            }
            let bytes = std::slice::from_raw_parts(
                (*credential).CredentialBlob,
                (*credential).CredentialBlobSize as usize,
            )
            .to_vec();
            CredFree(credential as _);
            bytes
        };
        Ok(Some(bytes))
    }
    pub fn delete(name: &str) -> Result<(), String> {
        if unsafe { CredDeleteW(target(name).as_ptr(), CRED_TYPE_GENERIC, 0) } == 0
            && std::io::Error::last_os_error().raw_os_error() != Some(ERROR_NOT_FOUND as i32)
        {
            return Err(
                "Windows non riesce a dimenticare l'account. Rimuovilo da Gestione credenziali"
                    .into(),
            );
        }
        Ok(())
    }
}

#[cfg(not(windows))]
mod native {
    pub fn write(_: &str, _: &[u8]) -> Result<(), String> {
        Err("Memorizzazione account disponibile solo su Windows".into())
    }
    pub fn read(_: &str) -> Result<Option<Vec<u8>>, String> {
        Ok(None)
    }
    pub fn delete(_: &str) -> Result<(), String> {
        Ok(())
    }
}
