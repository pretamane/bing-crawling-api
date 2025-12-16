//! Advanced Stealth & Obfuscation Module
//! 
//! This module provides:
//! - Advanced CDP (Chrome DevTools Protocol) evasion
//! - Realistic fingerprint spoofing (Canvas, WebGL, Audio, Fonts)
//! - Behavioral emulation scripts
//! - Randomized hardware profiles

use once_cell::sync::Lazy;
use rand::seq::SliceRandom;

/// Generate the main stealth injection script
/// This script runs before any other script on the page (via Page.addScriptToEvaluateOnNewDocument)
pub fn get_stealth_script() -> String {
    // We construct the script dynamically to allow for randomization per session
    
    let base_script = r#"
        // ============================================================================
        // 🛡️ ANTI-FINGERPRINTING & HARDENING (Tier 1)
        // ============================================================================

        // 1. Unmasking: Remove `navigator.webdriver`
        Object.defineProperty(navigator, 'webdriver', {
            get: () => undefined,
        });

        // 2. Hardware Concurrency Spoofing (Randomize 4-16)
        Object.defineProperty(navigator, 'hardwareConcurrency', {
            get: () => 4 + Math.floor(Math.random() * 4) * 2, // 4, 6, 8, 10...
        });

        // 3. Memory Spoofing (Randomize 4-32 GB)
        Object.defineProperty(navigator, 'deviceMemory', {
            get: () => 4 + Math.floor(Math.random() * 4) * 4, // 4, 8, 16, 24...
        });

        // 4. Chrome Runtime Mocking (Essential for "headless" checks)
        window.chrome = {
            runtime: {
                // Mock extension connection
                connect: function() {
                    return {
                        onMessage: {
                            addListener: function() {},
                            removeListener: function() {}
                        },
                        postMessage: function() {},
                        disconnect: function() {}
                    };
                },
                sendMessage: function() {},
                onMessage: {
                    addListener: function() {},
                    removeListener: function() {}
                },
                id: "pkghijhgljhglijhglijhglijhglij" // Random-looking ID
            },
            app: {
                isInstalled: false,
                InstallState: {
                    DISABLED: "disabled",
                    INSTALLED: "installed",
                    NOT_INSTALLED: "not_installed"
                },
                RunningState: {
                    CANNOT_RUN: "cannot_run",
                    READY_TO_RUN: "ready_to_run",
                    RUNNING: "running"
                }
            },
            csi: function() {},
            loadTimes: function() {
                return {
                    getLoadTime: () => Math.random(),
                    getStartLoadTime: () => Date.now() - (Math.random() * 1000),
                    commitLoadTime: Math.random(),
                    finishDocumentLoadTime: Math.random() * 10,
                    finishLoadTime: Math.random() * 10,
                    firstPaintAfterLoadTime: 0,
                    firstPaintTime: Math.random(),
                    navigationType: "Other",
                    wasFetchedViaSpdy: true,
                    wasNpnNegotiated: true,
                    npnNegotiatedProtocol: "h2",
                    wasAlternateProtocolAvailable: false,
                    connectionInfo: "h2"
                };
            }
        };

        // 5. Permission Mocking (Notifications = default/denied, not 'prompt')
        const originalQuery = window.navigator.permissions.query;
        window.navigator.permissions.query = (parameters) => (
            parameters.name === 'notifications' ?
            Promise.resolve({ state: Notification.permission }) :
            originalQuery(parameters)
        );
        
        // 6. WebRTC IP Leak Prevention (Disable or Mask)
        // Some sites check if WebRTC is completely missing to detect bots.
        // Better to mock it or leave it but ensure it doesn't leak local IP.
        // For now, we disable it as it's the safest 'nuclear' option against IP leaks.
        ['RTCPeerConnection', 'webkitRTCPeerConnection', 'mozRTCPeerConnection', 'msRTCPeerConnection'].forEach(className => {
             if (window[className]) {
                 window[className] = undefined;
             }
        });

        // ============================================================================
        // 🎨 FINGERPRINT SPOOFING (Tier 2 - Canvas/WebGL/Audio)
        // ============================================================================

        // 7. Canvas Noise (Perlin-like jitter)
        const originalToDataURL = HTMLCanvasElement.prototype.toDataURL;
        HTMLCanvasElement.prototype.toDataURL = function(...args) {
            // Only inject noise if the canvas is large enough to be a fingerprint attempt
            if (this.width > 16 && this.height > 16) {
                const context = this.getContext('2d');
                if (context) {
                    const imageData = context.getImageData(0, 0, this.width, this.height);
                    // Single pixel alpha modification isn't reliable enough, we need scattered noise
                    for (let i = 0; i < 5; i++) {
                         const x = Math.floor(Math.random() * this.width);
                         const y = Math.floor(Math.random() * this.height);
                         const idx = (y * this.width + x) * 4;
                         // Tweaking alpha channel slightly
                         if (imageData.data[idx+3] > 0) {
                             imageData.data[idx+3] = Math.max(0, Math.min(255, imageData.data[idx+3] + (Math.random() > 0.5 ? 1 : -1)));
                         }
                    }
                    context.putImageData(imageData, 0, 0);
                }
            }
            return originalToDataURL.apply(this, args);
        };

        // 8. WebGL Vendor Spoofing
        const getParameter = WebGLRenderingContext.prototype.getParameter;
        WebGLRenderingContext.prototype.getParameter = function(parameter) {
            // UNMASKED_VENDOR_WEBGL
            if (parameter === 37445) return 'Intel Inc.';
            // UNMASKED_RENDERER_WEBGL
            if (parameter === 37446) return 'Intel Iris OpenGL Engine';
            return getParameter.apply(this, [parameter]);
        };

        // 9. AudioContext Noise (Audio Fingerprint Defense)
        const originalCreateOscillator = window.AudioContext.prototype.createOscillator || window.webkitAudioContext.prototype.createOscillator;
        if (originalCreateOscillator) {
            const contextProto = window.AudioContext ? window.AudioContext.prototype : window.webkitAudioContext.prototype;
            contextProto.createOscillator = function() {
                const oscillator = originalCreateOscillator.apply(this, arguments);
                const originalStart = oscillator.start;
                oscillator.start = function(when = 0) {
                    // Micro-jitter to frequency/start time
                    return originalStart.apply(this, [when + (Math.random() * 0.00001)]);
                };
                return oscillator;
            };
        }

        // ============================================================================
        // 🔌 PLUGINS & MIMETYPES (Tier 3)
        // ============================================================================

        // 10. Spoof Plugins (Standard Chrome Set)
        Object.defineProperty(navigator, 'plugins', {
            get: () => {
                const pdf = {
                    0: { type: "application/x-google-chrome-pdf", suffixes: "pdf", description: "Portable Document Format" },
                    description: "Portable Document Format",
                    filename: "internal-pdf-viewer",
                    length: 1,
                    name: "Chrome PDF Plugin"
                };
                const p = [pdf, pdf, pdf, pdf, pdf];
                Object.setPrototypeOf(p, PluginArray.prototype);
                return p;
            }
        });

        // 11. Spoof MimeTypes
        Object.defineProperty(navigator, 'mimeTypes', {
            get: () => {
                const pdfMime = {
                    type: "application/pdf",
                    suffixes: "pdf",
                    description: "",
                    enabledPlugin: navigator.plugins[0]
                };
                const m = [pdfMime];
                Object.setPrototypeOf(m, MimeTypeArray.prototype);
                return m;
            }
        });

        // ============================================================================
        // 🕵️ EXTRA EVASION
        // ============================================================================

        // 12. Broken Image Detection Override
        // Some bots are detected because they don't load images. 
        // We ensure 'natural' behavior attributes are present.
        Object.defineProperty(HTMLImageElement.prototype, 'naturalWidth', {
             get: function() { return this.width > 0 ? this.width : 1; } 
        });
        Object.defineProperty(HTMLImageElement.prototype, 'naturalHeight', {
             get: function() { return this.height > 0 ? this.height : 1; } 
        });

        console.log("🛡️ Stealth Injection Complete");
    "#;

    base_script.to_string()
}

/// JS to simulate realistic human mouse movement
pub const MOUSE_MOVE_JS: &str = r#"
    function bezier(t, p0, p1, p2, p3) {
        const cX = 3 * (p1.x - p0.x), bX = 3 * (p2.x - p1.x) - cX, aX = p3.x - p0.x - cX - bX;
        const cY = 3 * (p1.y - p0.y), bY = 3 * (p2.y - p1.y) - cY, aY = p3.y - p0.y - cY - bY;
        const x = (aX * Math.pow(t, 3)) + (bX * Math.pow(t, 2)) + (cX * t) + p0.x;
        const y = (aY * Math.pow(t, 3)) + (bY * Math.pow(t, 2)) + (cY * t) + p0.y;
        return {x: x, y: y};
    }

    async function humanMouseMove(startX, startY, endX, endY, steps) {
        // Random control points for Bezier curve (Natural Arc)
        const p0 = {x: startX, y: startY};
        const p3 = {x: endX, y: endY};
        // Control points randomized to create "arc" or "swerve"
        const p1 = {
            x: startX + (Math.random() * (endX - startX)) + (Math.random() * 100 - 50), 
            y: startY + (Math.random() * (endY - startY)) + (Math.random() * 100 - 50)
        };
        const p2 = {
            x: startX + (Math.random() * (endX - startX)) + (Math.random() * 100 - 50), 
            y: startY + (Math.random() * (endY - startY)) + (Math.random() * 100 - 50)
        };

        for (let i = 0; i <= steps; i++) {
            const t = i / steps;
            const pos = bezier(t, p0, p1, p2, p3);
            
            // Dispatch multiple events for realism
            document.dispatchEvent(new MouseEvent('mousemove', {
                view: window,
                bubbles: true,
                cancelable: true,
                clientX: pos.x,
                clientY: pos.y,
                screenX: pos.x + 100, // Simulate screen vs client diff
                screenY: pos.y + 100
            }));
            
            // Non-linear timing (faster in middle, slower at ends)
            // const wait = 5 + Math.abs(Math.sin(t * Math.PI)) * 10; 
            await new Promise(r => setTimeout(r, 5 + Math.random() * 5));
        }
    }
"#;

/// JS to simulate realistic scrolling
pub const SCROLL_JS: &str = r#"
    async function humanScroll(targetY) {
        let currentY = window.scrollY;
        while (Math.abs(currentY - targetY) > 10) {
            // Speed varies
            const step = (targetY - currentY) * (0.05 + Math.random() * 0.05);
            window.scrollBy(0, step);
            currentY = window.scrollY;
            
            // Occasional "pause" to read
            if (Math.random() < 0.05) {
                await new Promise(r => setTimeout(r, 200 + Math.random() * 300));
            }
            
            await new Promise(r => setTimeout(r, 10 + Math.random() * 20));
        }
    }
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stealth_script_generation() {
        let script = get_stealth_script();
        assert!(script.contains("Object.defineProperty(navigator, 'webdriver'"));
        assert!(script.contains("window.chrome = {"));
        assert!(script.contains("HTMLCanvasElement.prototype.toDataURL"));
        println!("Stealth script generated successfully, length: {}", script.len());
    }
}

// ============================================================================
// 🖱️ NATIVE HUMAN INPUT SIMULATION (Rust-Side)
// ============================================================================

use headless_chrome::{Tab, protocol::cdp::{Input::{DispatchMouseEvent, DispatchMouseEventTypeOption, DispatchMouseEventPointer_TypeOption}, Emulation::{SetTimezoneOverride, SetLocaleOverride}}};
use anyhow::Result;
use rand::Rng;

#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// Calculate a point on a cubic Bezier curve
fn cubic_bezier(t: f64, p0: Point, p1: Point, p2: Point, p3: Point) -> Point {
    let cx = 3.0 * (p1.x - p0.x);
    let bx = 3.0 * (p2.x - p1.x) - cx;
    let ax = p3.x - p0.x - cx - bx;

    let cy = 3.0 * (p1.y - p0.y);
    let by = 3.0 * (p2.y - p1.y) - cy;
    let ay = p3.y - p0.y - cy - by;

    let x = (ax * t.powi(3)) + (bx * t.powi(2)) + (cx * t) + p0.x;
    let y = (ay * t.powi(3)) + (by * t.powi(2)) + (cy * t) + p0.y;
    Point { x, y }
}

/// Simulate human-like mouse movement using CDP (Trusted Events)
pub async fn move_mouse_human(tab: &std::sync::Arc<Tab>, start: Point, end: Point) -> Result<()> {
    let steps = 25;
    
    // Random control points for a natural arc
    // p1 and p2 control the "swerve" of the curve
    let p0 = start;
    let p3 = end;
    
    let variance = 100.0;
    let (p1, p2) = {
        let mut rng = rand::thread_rng();
        let p1 = Point::new(
            start.x + (end.x - start.x) * rng.gen_range(0.2..0.8) + rng.gen_range(-variance..variance),
            start.y + (end.y - start.y) * rng.gen_range(0.2..0.8) + rng.gen_range(-variance..variance),
        );
        let p2 = Point::new(
            start.x + (end.x - start.x) * rng.gen_range(0.2..0.8) + rng.gen_range(-variance..variance),
            start.y + (end.y - start.y) * rng.gen_range(0.2..0.8) + rng.gen_range(-variance..variance),
        );
        (p1, p2)
    };

    for i in 0..=steps {
        let t = i as f64 / steps as f64;
        
        let p = cubic_bezier(t, p0, p1, p2, p3);

        // Dispatch Native Event via CDP
        tab.call_method(DispatchMouseEvent {
            x: p.x,
            y: p.y,
            Type: DispatchMouseEventTypeOption::MouseMoved,
            button: None,
            buttons: None,
            modifiers: None,
            timestamp: None,
            delta_x: None,
            delta_y: None,
            pointer_Type: Some(DispatchMouseEventPointer_TypeOption::Mouse),
            force: None,
            tangential_pressure: None,
            tilt_x: None,
            tilt_y: None,
            twist: None,
            click_count: None,
        })?;

        // Sleep to simulate movement speed
        // Randomize sleep: 5ms to 15ms
        let delay = rand::thread_rng().gen_range(5..15);
        tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
    }
    
    Ok(())
}

/// Move mouse to a specific element's center (with randomization)
pub async fn move_mouse_to_element(tab: &std::sync::Arc<Tab>, selector: &str) -> Result<()> {
    let element = tab.wait_for_element(selector)?;
    let box_model = element.get_box_model()?;
    
    // Get center of element content
    // ElementQuad does not have iter(), so we average manually or use box logic
    // We'll use the center of the content box
    let center_x = (box_model.content.top_left.x + box_model.content.top_right.x + box_model.content.bottom_right.x + box_model.content.bottom_left.x) / 4.0;
    let center_y = (box_model.content.top_left.y + box_model.content.top_right.y + box_model.content.bottom_right.y + box_model.content.bottom_left.y) / 4.0;
    
    // Starting position? For now, we assume current position or (0,0) if unknown
    // In a real flow, we should track the last known mouse position. 
    // For this implementation, we'll start from a random offset or (100,100)
    let start = Point::new(100.0, 100.0); 
    let end = Point::new(center_x, center_y);
    
    move_mouse_human(tab, start, end).await?;
    Ok(())
}

/// Simulate human-like scrolling using CDP (Trusted Events)
pub async fn scroll_human(tab: &std::sync::Arc<Tab>, delta_y: f64) -> Result<()> {
    let steps = 10;
    let step_size = delta_y / steps as f64;

    for _ in 0..steps {
        tab.call_method(DispatchMouseEvent {
            Type: DispatchMouseEventTypeOption::MouseWheel,
            x: 100.0,
            y: 100.0,
            button: None,
            buttons: None,
            modifiers: None,
            timestamp: None,
            delta_x: Some(0.0),
            delta_y: Some(step_size),
            pointer_Type: Some(DispatchMouseEventPointer_TypeOption::Mouse),
            force: None,
            tangential_pressure: None,
            tilt_x: None,
            tilt_y: None,
            twist: None,
            click_count: None,
        })?;

        let delay = rand::thread_rng().gen_range(50..150);
        tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
    }
    
    Ok(())
}

/// Apply fingerprint overrides (Timezone, Locale) to match IP
pub async fn apply_stealth_settings(tab: &std::sync::Arc<Tab>, timezone_id: &str, locale: &str) -> anyhow::Result<()> {
    // Override Timezone (e.g., "Asia/Yangon")
    tab.call_method(SetTimezoneOverride {
        timezone_id: timezone_id.to_string(),
    })?;

    // Override Locale (e.g., "en-US,en" or "my-MM")
    // Most users use en-US even abroad, but the Timezone MUST match the IP.
    tab.call_method(SetLocaleOverride {
        locale: Some(locale.to_string()),
    })?;

    Ok(())
}
