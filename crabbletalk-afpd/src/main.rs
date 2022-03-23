use anyhow::Result;

const ADAPTERS: &'static str =
    r"SYSTEM\CurrentControlSet\Control\Class\{4D36E972-E325-11CE-BFC1-08002BE10318}";

fn get_adapter(adapters: &winreg::RegKey, adapter_key: &str) -> Option<winreg::RegValue> {
    let adapter = adapters.open_subkey(adapter_key).ok()?;
    let component_id: String = adapter.get_value("ComponentId").ok()?;
    println!("lesse.. {:?}", component_id);
    if component_id != r"root\tap0901" {
        return None;
    }
    println!("so what else we got..");
    for kv_r in adapter.enum_values() {
        println!("well.. {:?}", kv_r);
    }
    adapter.get_raw_value("NetCfgInstanceId").ok()
}

#[tokio::main]
async fn main() -> Result<()> {
    let adapters =
        winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE).open_subkey(ADAPTERS)?;
    let adapter_uuid = adapters.enum_keys().map(|adapter_key_r| -> Result<winreg::RegValue> {
        let adapter_key = adapter_key_r?;
        match get_adapter(&adapters, &adapter_key) {
            Some(u) => Ok(u),
            None => Err(anyhow::anyhow!("haha hi")),
        }
    }).collect::<Result<_>>()?;
    
    Ok(())
}
