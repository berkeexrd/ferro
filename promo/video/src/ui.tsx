import React from "react";
import { AbsoluteFill, Easing, interpolate, spring, staticFile, useCurrentFrame, useVideoConfig } from "remotion";

export const C = {
  bg0: "#151412",
  bg1: "#1B1A17",
  bg2: "#201F1B",
  bg3: "#2A2824",
  line: "#36332D",
  text1: "#ECE8E1",
  text2: "#B1AAA0",
  text3: "#9A9388",
  accent: "#E58A55",
  accentHi: "#F0A173",
  blue: "#7FB5F2",
  teal: "#86CBC6",
  purple: "#B79CF5",
  green: "#7FD3A5",
  red: "#F29B9B",
  yellow: "#E8BE55",
};

export const fontCss = `
@font-face{font-family:'Inter';src:url(${staticFile("fonts/Inter-Regular.otf")});font-weight:400}
@font-face{font-family:'Inter';src:url(${staticFile("fonts/Inter-SemiBold.otf")});font-weight:600}
@font-face{font-family:'JB';src:url(${staticFile("fonts/JetBrainsMono-Regular.ttf")});font-weight:400}
@font-face{font-family:'JB';src:url(${staticFile("fonts/JetBrainsMono-Bold.ttf")});font-weight:700}
`;

export const ease = Easing.bezier(0.22, 1, 0.36, 1);

/** Fades a scene in and out over `n` frames. */
export const useFade = (dur: number, nIn = 14, nOut = nIn) => {
  const f = useCurrentFrame();
  return interpolate(f, [0, nIn, dur - nOut, dur], [0, 1, 1, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
};

export const Background: React.FC<{ intensity?: number }> = ({ intensity = 1 }) => {
  const f = useCurrentFrame();
  const t = f / 30;
  return (
    <AbsoluteFill style={{ background: C.bg0 }}>
      <AbsoluteFill
        style={{
          background: `radial-gradient(900px 700px at ${22 + Math.sin(t * 0.35) * 6}% ${86 + Math.cos(t * 0.3) * 4}%, rgba(229,138,85,${0.22 * intensity}), transparent 70%),
                       radial-gradient(800px 600px at ${82 + Math.cos(t * 0.3) * 5}% ${8 + Math.sin(t * 0.4) * 4}%, rgba(127,181,242,${0.12 * intensity}), transparent 70%),
                       radial-gradient(600px 500px at 60% 60%, rgba(183,156,245,${0.05 * intensity}), transparent 70%)`,
        }}
      />
      <AbsoluteFill style={{ opacity: 0.5, backgroundImage: "radial-gradient(rgba(255,255,255,0.045) 1px, transparent 1px)", backgroundSize: "34px 34px" }} />
    </AbsoluteFill>
  );
};

/** Words rise in one by one. */
export const Kinetic: React.FC<{ text: string; size?: number; delay?: number; color?: string; weight?: number; gap?: number; align?: "left" | "center" }> = ({
  text,
  size = 96,
  delay = 0,
  color = C.text1,
  weight = 600,
  gap = 5,
  align = "left",
}) => {
  const f = useCurrentFrame();
  const { fps } = useVideoConfig();
  const words = text.split(" ");
  return (
    <div style={{ display: "flex", flexWrap: "wrap", gap: `0 ${size * 0.26}px`, justifyContent: align === "center" ? "center" : "flex-start", fontFamily: "Inter", fontWeight: weight, fontSize: size, lineHeight: 1.08, letterSpacing: -size * 0.025, color }}>
      {words.map((w, i) => {
        const s = spring({ frame: f - delay - i * gap, fps, config: { damping: 18, mass: 0.7, stiffness: 120 } });
        return (
          <span key={i} style={{ display: "inline-block", opacity: s, transform: `translateY(${(1 - s) * 40}px)`, filter: `blur(${(1 - s) * 10}px)` }}>
            {w}
          </span>
        );
      })}
    </div>
  );
};

export const Eyebrow: React.FC<{ children: React.ReactNode; delay?: number }> = ({ children, delay = 0 }) => {
  const f = useCurrentFrame();
  const o = interpolate(f - delay, [0, 14], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  return (
    <div style={{ opacity: o, fontFamily: "JB", fontWeight: 700, fontSize: 22, letterSpacing: 6, color: C.accent, textTransform: "uppercase", marginBottom: 22 }}>{children}</div>
  );
};

export const Sub: React.FC<{ children: React.ReactNode; delay?: number; size?: number; width?: number }> = ({ children, delay = 0, size = 30, width = 900 }) => {
  const f = useCurrentFrame();
  const o = interpolate(f - delay, [0, 18], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const y = interpolate(f - delay, [0, 18], [18, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp", easing: ease });
  return <div style={{ opacity: o, transform: `translateY(${y}px)`, fontFamily: "Inter", fontSize: size, lineHeight: 1.45, color: C.text2, maxWidth: width, marginTop: 28 }}>{children}</div>;
};

/** App window shell around a screenshot / clip. */
export const Win: React.FC<{ width: number; children: React.ReactNode; style?: React.CSSProperties; glow?: boolean }> = ({ width, children, style, glow = true }) => (
  <div
    style={{
      width,
      height: (width * 1040) / 1920,
      borderRadius: 18,
      overflow: "hidden",
      position: "relative",
      background: C.bg2,
      boxShadow: `0 0 0 1px rgba(255,255,255,0.09), 0 50px 120px rgba(0,0,0,0.65)${glow ? ", 0 0 160px rgba(229,138,85,0.14)" : ""}`,
      ...style,
    }}
  >
    {children}
  </div>
);

export const Chip: React.FC<{ children: React.ReactNode; color?: string; style?: React.CSSProperties }> = ({ children, color = C.accent, style }) => (
  <div
    style={{
      display: "inline-flex",
      alignItems: "center",
      gap: 12,
      padding: "12px 22px",
      borderRadius: 999,
      background: "rgba(27,26,23,0.82)",
      border: `1px solid ${color}66`,
      boxShadow: `0 10px 40px rgba(0,0,0,0.5), 0 0 30px ${color}22`,
      fontFamily: "Inter",
      fontWeight: 600,
      fontSize: 26,
      color: C.text1,
      backdropFilter: "blur(10px)",
      ...style,
    }}
  >
    <span style={{ width: 12, height: 12, borderRadius: 6, background: color, boxShadow: `0 0 14px ${color}` }} />
    {children}
  </div>
);

export const Key: React.FC<{ children: React.ReactNode }> = ({ children }) => (
  <span style={{ fontFamily: "JB", fontWeight: 700, fontSize: 24, padding: "6px 14px", borderRadius: 9, border: `1.5px solid ${C.line}`, background: "rgba(21,20,18,0.85)", color: C.text1, boxShadow: "0 3px 0 rgba(0,0,0,0.5)", marginLeft: 8 }}>
    {children}
  </span>
);

export const FMark: React.FC<{ size: number }> = ({ size }) => (
  <div style={{ width: size, height: size, filter: `drop-shadow(0 ${size * 0.08}px ${size * 0.22}px rgba(0,0,0,0.6)) drop-shadow(0 0 ${size * 0.18}px rgba(229,138,85,0.35))` }}>
    <img src={staticFile("img/logo.png")} style={{ width: "100%", height: "100%" }} />
  </div>
);
