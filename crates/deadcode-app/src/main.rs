mod app;
mod modding;

use winit::event_loop::EventLoop;

use app::App;
use deadcode_desktop::UserEvent;

fn main() {
    #[cfg(target_os = "windows")]
    webview2_check::ensure_webview2();

    let event_loop = EventLoop::<UserEvent>::with_user_event()
        .build()
        .expect("Failed to create event loop");

    let proxy = event_loop.create_proxy();
    let mut app = App::new(proxy);
    event_loop.run_app(&mut app).expect("Event loop failed");
}

#[cfg(target_os = "windows")]
mod webview2_check {
    use std::process::Command;

    pub fn ensure_webview2() {
        if is_webview2_installed() {
            return;
        }

        eprintln!("WebView2 runtime not found — downloading and installing...");

        // Single PowerShell command: download bootstrapper, run elevated, wait, clean up.
        let script = r#"
$ErrorActionPreference = 'Stop'
$out = Join-Path $env:TEMP 'MicrosoftEdgeWebview2Setup.exe'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
(New-Object Net.WebClient).DownloadFile('https://go.microsoft.com/fwlink/p/?LinkId=2124703', $out)
Start-Process -FilePath $out -ArgumentList '/silent','/install' -Verb RunAs -Wait
Remove-Item $out -ErrorAction SilentlyContinue
"#;

        let status = Command::new("powershell")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", script])
            .status();

        match status {
            Ok(s) if s.success() => {
                if is_webview2_installed() {
                    eprintln!("WebView2 runtime installed successfully.");
                } else {
                    eprintln!(
                        "Installation may have been cancelled. \
                         Please install WebView2 manually from: \
                         https://developer.microsoft.com/en-us/microsoft-edge/webview2/"
                    );
                }
            }
            Ok(s) => eprintln!("WebView2 installer exited with: {s}"),
            Err(e) => eprintln!("Failed to run installer: {e}"),
        }
    }

    fn is_webview2_installed() -> bool {
        for key in [
            r"HKLM\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
            r"HKLM\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
        ] {
            if let Ok(out) = Command::new("reg").args(["query", key, "/v", "pv"]).output() {
                if out.status.success() {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    if !stdout.contains("0.0.0.0") {
                        return true;
                    }
                }
            }
        }
        false
    }
}
