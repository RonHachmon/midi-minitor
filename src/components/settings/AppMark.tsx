import markUrl from "../../assets/neon-dj-headset-waveform.svg";

/**
 * The application's mark: the neon headset the window and the icon both carry.
 *
 * # Why there is no plate behind it
 *
 * There was one, on the assumption that neon needs a dark ground. Rendering the
 * artwork over this window's warm off-white and over white proved that wrong at
 * this size: the glow is a soft bloom rather than a haze, and the strokes are
 * saturated enough to hold their own. The plate was solving a problem the
 * drawing does not have here, while introducing one it does — a dark slab in the
 * middle of a light preferences pane. Transparent is both simpler and truer to
 * the artwork.
 *
 * The glow *is* load-bearing at icon sizes, which is why `app-icon.svg` exists
 * as a separate, coarser cut rather than this file being rasterised directly.
 *
 * # Why this is here rather than on a surface the screenshots depict
 *
 * `screenshots/setting.jpg` shows that a `Sources` tab exists but not what is
 * inside it, so its contents are surface the reference never fixed — which is
 * exactly where the constitution allows something new to go. Nothing the
 * screenshots do depict gains a logo: not the monitor, not the `Display` tab,
 * not the window chrome.
 *
 * # Why it is an `img` rather than inline markup
 *
 * The file is the single source for the mark, so the webview points at it rather
 * than holding a second copy that could drift. CSS animations declared inside an
 * SVG still run when it is loaded this way, and the artwork carries its own
 * `prefers-reduced-motion` rule, so a viewer who has asked for stillness gets it
 * without this component knowing anything about that.
 */
export function AppMark() {
  return (
    <img
      src={markUrl}
      alt="MIDI minitor"
      width={260}
      height={187}
      className="block h-auto w-[260px]"
    />
  );
}
