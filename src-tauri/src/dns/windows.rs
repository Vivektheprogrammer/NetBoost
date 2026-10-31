use crate::storage::db::{Database, DnsBackupRecord};
use std::os::windows::process::CommandExt;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const CREATE_NO_WINDOW: u32 = 0x08000000;

pub struct WindowsDnsManager;

impl WindowsDnsManager {
    /// Query current DNS server configuration for a specific interface via netsh
    pub fn get_interface_dns_config(iface: &str) -> (bool, Option<String>, Option<String>) {
        let output = match Command::new("netsh")
            .args(["interface", "ipv4", "show", "dnsservers", &format!("name={}", iface)])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
        {
            Ok(out) => String::from_utf8_lossy(&out.stdout).to_string(),
            Err(_) => return (true, None, None),
        };

        let is_dhcp = output.contains("DHCP") || output.contains("configured through DHCP");

        let mut ips = Vec::new();
        for line in output.lines() {
            let trimmed = line.trim();
            // Look for IPv4 addresses
            for part in trimmed.split_whitespace() {
                if let Ok(ip) = part.parse::<std::net::Ipv4Addr>() {
                    let s = ip.to_string();
                    if !ips.contains(&s) && s != "0.0.0.0" {
                        ips.push(s);
                    }
                }
            }
        }

        let primary = ips.get(0).cloned();
        let secondary = ips.get(1).cloned();

        (is_dhcp, primary, secondary)
    }

    /// Automatically backup current DNS configuration before applying changes
    pub fn backup_dns_if_needed(db: &Database, iface: &str) -> Result<DnsBackupRecord, String> {
        if let Ok(Some(existing)) = db.get_dns_backup(iface) {
            return Ok(existing);
        }

        let (is_dhcp, primary, secondary) = Self::get_interface_dns_config(iface);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let rec = DnsBackupRecord {
            interface_name: iface.to_string(),
            is_dhcp,
            primary_dns: primary,
            secondary_dns: secondary,
            backed_up_at: now,
        };

        db.save_dns_backup(&rec)
            .map_err(|e| format!("Failed to persist original DNS backup: {}", e))?;

        Ok(rec)
    }

    /// Apply user-confirmed DNS servers to the network adapter
    pub fn apply_dns(
        db: &Database,
        iface: &str,
        primary_ip: &str,
        secondary_ip: Option<&str>,
    ) -> Result<(), String> {
        // Step 1: Ensure original DNS is backed up first
        let _ = Self::backup_dns_if_needed(db, iface)?;

        // Step 2: Set primary DNS static address
        run_netsh_cmd(&[
            "interface",
            "ipv4",
            "set",
            "dnsservers",
            &format!("name={}", iface),
            "static",
            primary_ip,
            "primary",
        ])?;

        // Step 3: Add secondary DNS server if specified
        if let Some(sec) = secondary_ip {
            if !sec.is_empty() && sec != "—" {
                let _ = run_netsh_cmd(&[
                    "interface",
                    "ipv4",
                    "add",
                    "dnsservers",
                    &format!("name={}", iface),
                    sec,
                    "index=2",
                ]);
            }
        }

        Ok(())
    }

    /// One-click restoration of original DNS settings
    pub fn restore_original_dns(db: &Database, iface: &str) -> Result<(), String> {
        let backup = match db.get_dns_backup(iface) {
            Ok(Some(rec)) => rec,
            _ => {
                // If no record found, default to restoring DHCP automatic resolution
                DnsBackupRecord {
                    interface_name: iface.to_string(),
                    is_dhcp: true,
                    primary_dns: None,
                    secondary_dns: None,
                    backed_up_at: 0,
                }
            }
        };

        if backup.is_dhcp {
            run_netsh_cmd(&[
                "interface",
                "ipv4",
                "set",
                "dnsservers",
                &format!("name={}", iface),
                "source=dhcp",
            ])?;
        } else if let Some(ref pri) = backup.primary_dns {
            run_netsh_cmd(&[
                "interface",
                "ipv4",
                "set",
                "dnsservers",
                &format!("name={}", iface),
                "static",
                pri,
                "primary",
            ])?;

            if let Some(ref sec) = backup.secondary_dns {
                if !sec.is_empty() && sec != "—" {
                    let _ = run_netsh_cmd(&[
                        "interface",
                        "ipv4",
                        "add",
                        "dnsservers",
                        &format!("name={}", iface),
                        sec,
                        "index=2",
                    ]);
                }
            }
        }

        // Clean up backup after successful restoration
        let _ = db.delete_dns_backup(iface);
        Ok(())
    }
}

/// Executes a netsh command silently with CREATE_NO_WINDOW and handles UAC elevation cleanly
fn run_netsh_cmd(args: &[&str]) -> Result<(), String> {
    let mut cmd = Command::new("netsh");
    cmd.args(args).creation_flags(CREATE_NO_WINDOW);

    match cmd.output() {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => {
            let stderr_str = String::from_utf8_lossy(&out.stderr).to_string();
            let stdout_str = String::from_utf8_lossy(&out.stdout).to_string();
            let combined = format!("{} {}", stderr_str.trim(), stdout_str.trim()).trim().to_string();

            // If unprivileged netsh reports elevation requirement, invoke with RunAs cleanly
            if combined.contains("elevation") || combined.contains("administrator") || combined.contains("Access is denied") {
                let arg_string = args.join(" ");
                let ps_script = format!(
                    "Start-Process netsh -ArgumentList '{}' -Verb RunAs -WindowStyle Hidden -Wait",
                    arg_string
                );
                let mut elevated_cmd = Command::new("powershell");
                elevated_cmd.args(["-NoProfile", "-NonInteractive", "-Command", &ps_script]);
                elevated_cmd.creation_flags(CREATE_NO_WINDOW);

                if let Ok(elevated_out) = elevated_cmd.output() {
                    if elevated_out.status.success() {
                        return Ok(());
                    }
                }

                return Err("Modifying network adapter DNS settings requires Windows Administrator permission. Please allow the UAC prompt or run NetBoost as administrator.".to_string());
            }

            if !combined.is_empty() {
                Err(combined)
            } else {
                Err("Operation failed. Ensure network adapter is connected.".to_string())
            }
        }
        Err(e) => Err(format!("Failed to execute netsh: {}", e)),
    }
}

