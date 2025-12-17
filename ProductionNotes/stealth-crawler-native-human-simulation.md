# Native Human Simulation (True Stealth)

To achieve "Legit Powerful Crawling", we must move away from JavaScript-injected events (`document.dispatchEvent`). These events usually have the `isTrusted` property set to `false`, which sophisticated anti-bot systems (like Google/Bing/Cloudflare) can easily detect.

Instead, we will implement the simulation on the **Rust** side, sending `Input.dispatchMouseEvent` commands directly to the browser process via the Chrome DevTools Protocol (CDP). These events appear as genuine hardware inputs (`isTrusted: true`).

## Proposed Changes

### [MODIFY] [stealth.rs](file:///home/guest/tzdump/crawling/rust-crawler/src/stealth.rs)
Add a new struct `HumanInput` or helper functions that perform the following:
1.  **Bezier Curve Calculation**: Implement the math in Rust to generate a series of (x, y) points representing a natural curve.
2.  **Physics-based Timing**: Calculate delays between points based on "speed" and "acceleration/deceleration" near targets (Fitts's Law).
3.  **CDP Execution**: Use `headless_chrome::Tab` to execute `move_mouse_to_point` sequentially.

### [MODIFY] [crawler.rs](file:///home/guest/tzdump/crawling/rust-crawler/src/crawler.rs)
- Remove the JS `humanMouseMove` calls.
- Instead, find the **bounding box** of target elements (e.g., search bar, result link) using `tab.find_element(...).get_box_model()`.
- Call the new `stealth::move_mouse_to_element(tab, element)` function.
- This creates a dynamic, non-deterministic path to the *actual* UI element, regardless of screen resolution.

## Verification Plan

### Live Verification
- Run the crawler against Google.
- Observe behavior (logs) showing "Moving mouse from (X,Y) to Element at (X,Y)".
- Verify success rate (no immediate blocks).
