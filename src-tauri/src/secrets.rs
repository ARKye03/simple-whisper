use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use base64::Engine;
use hkdf::Hkdf;
use rand::RngCore;
use serde_json::Value;
use sha2::Sha256;
use tauri::{AppHandle, Manager};

const SECRETS_FILE: &str = "secrets.json";
const DEVICE_KEY_FILE: &str = "device_key.bin";
const HKDF_SALT: &[u8] = b"simple-whisper:v1";
const HKDF_INFO: &[u8] = b"api-key-encryption";
const KEYCHAIN_SERVICE: &str = "com.arkye03.simple-whisper";

fn account_for(provider: &str) -> Result<&'static str, String> {
    match provider {
        "groq" => Ok("groq_api_key"),
        "gemini" => Ok("gemini_api_key"),
        other => Err(format!("Proveedor desconocido: {other}")),
    }
}

fn ensure_data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_local_data_dir()
        .map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn secrets_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(ensure_data_dir(app)?.join(SECRETS_FILE))
}

fn device_key_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(ensure_data_dir(app)?.join(DEVICE_KEY_FILE))
}

fn lock_file_perms(_path: &std::path::Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(_path, fs::Permissions::from_mode(0o600));
    }
}

fn load_or_create_fallback_key(app: &AppHandle) -> Result<[u8; 32], String> {
    let path = device_key_path(app)?;
    if path.exists() {
        let bytes = fs::read(&path).map_err(|e| e.to_string())?;
        if bytes.len() == 32 {
            let mut k = [0u8; 32];
            k.copy_from_slice(&bytes);
            return Ok(k);
        }
    }
    let mut k = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut k);
    fs::write(&path, k).map_err(|e| e.to_string())?;
    lock_file_perms(&path);
    Ok(k)
}

fn derive_key(app: &AppHandle) -> Result<[u8; 32], String> {
    match machine_uid::get() {
        Ok(uid) if !uid.is_empty() => {
            let hk = Hkdf::<Sha256>::new(Some(HKDF_SALT), uid.as_bytes());
            let mut out = [0u8; 32];
            hk.expand(HKDF_INFO, &mut out).map_err(|e| e.to_string())?;
            Ok(out)
        }
        _ => load_or_create_fallback_key(app),
    }
}

fn encrypt(key: &[u8; 32], plaintext: &str) -> Result<String, String> {
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let mut nonce_bytes = [0u8; 12];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ct = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| format!("cifrado falló: {e}"))?;
    let mut blob = Vec::with_capacity(12 + ct.len());
    blob.extend_from_slice(&nonce_bytes);
    blob.extend_from_slice(&ct);
    Ok(base64::engine::general_purpose::STANDARD.encode(blob))
}

fn decrypt(key: &[u8; 32], blob_b64: &str) -> Result<String, String> {
    let blob = base64::engine::general_purpose::STANDARD
        .decode(blob_b64)
        .map_err(|e| e.to_string())?;
    if blob.len() < 13 {
        return Err("blob cifrado inválido".into());
    }
    let (nonce_bytes, ct) = blob.split_at(12);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let pt = cipher
        .decrypt(Nonce::from_slice(nonce_bytes), ct)
        .map_err(|e| format!("descifrado falló: {e}"))?;
    String::from_utf8(pt).map_err(|e| e.to_string())
}

fn read_store(app: &AppHandle) -> Result<BTreeMap<String, String>, String> {
    let path = secrets_path(app)?;
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let data = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    if data.trim().is_empty() {
        return Ok(BTreeMap::new());
    }
    let v: Value = serde_json::from_str(&data).map_err(|e| e.to_string())?;
    let mut out = BTreeMap::new();
    if let Some(obj) = v.as_object() {
        for (k, val) in obj {
            if let Some(s) = val.as_str() {
                out.insert(k.clone(), s.to_string());
            }
        }
    }
    Ok(out)
}

fn write_store(app: &AppHandle, map: &BTreeMap<String, String>) -> Result<(), String> {
    let path = secrets_path(app)?;
    let json = serde_json::to_string_pretty(map).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())?;
    lock_file_perms(&path);
    Ok(())
}

fn get_secret(app: &AppHandle, provider: &str) -> Result<String, String> {
    let key_name = account_for(provider)?.to_string();
    let map = read_store(app)?;
    let Some(blob) = map.get(&key_name) else {
        return Ok(String::new());
    };
    let key = derive_key(app)?;
    // If decryption fails (e.g. device UID changed / corrupted file),
    // surface empty rather than booting into an error — user can re-enter.
    Ok(decrypt(&key, blob).unwrap_or_default())
}

fn set_secret(app: &AppHandle, provider: &str, value: &str) -> Result<(), String> {
    let key_name = account_for(provider)?.to_string();
    let trimmed = value.trim();
    let mut map = read_store(app)?;
    if trimmed.is_empty() {
        map.remove(&key_name);
    } else {
        let key = derive_key(app)?;
        let blob = encrypt(&key, trimmed)?;
        map.insert(key_name, blob);
    }
    write_store(app, &map)
}

#[tauri::command]
pub async fn secret_get(app: AppHandle, provider: String) -> Result<String, String> {
    get_secret(&app, &provider)
}

#[tauri::command]
pub async fn secret_set(app: AppHandle, provider: String, value: String) -> Result<(), String> {
    set_secret(&app, &provider, &value)
}

#[tauri::command]
pub async fn secret_migrate_from_keychain(
    app: AppHandle,
    provider: String,
) -> Result<bool, String> {
    let account = account_for(&provider)?;
    let Ok(entry) = keyring::Entry::new(KEYCHAIN_SERVICE, account) else {
        return Ok(false);
    };
    let password = match entry.get_password() {
        Ok(s) if !s.trim().is_empty() => s,
        _ => return Ok(false),
    };
    set_secret(&app, &provider, &password)?;
    let _ = entry.delete_credential();
    Ok(true)
}
