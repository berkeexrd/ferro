import React from "react";
import { AbsoluteFill, interpolate, spring, useCurrentFrame, useVideoConfig } from "remotion";
import { C, Kinetic, ease } from "./ui";

const BEAT = 30 * (60 / 139.7); // frames per beat

const Heart: React.FC<{ size: number }> = ({ size }) => {
  const f = useCurrentFrame();
  const ph = (f % BEAT) / BEAT;
  const pulse = 1 + 0.09 * Math.exp(-ph * 7);
  return (
    <svg width={size} height={size} viewBox="0 0 32 32" style={{ transform: `scale(${pulse})`, filter: `drop-shadow(0 0 ${size * 0.2}px rgba(229,138,85,0.55))` }}>
      <defs>
        <linearGradient id="hg" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor="#F0A173" />
          <stop offset="1" stopColor="#E0603A" />
        </linearGradient>
      </defs>
      <path d="M16 28.5C7 21.6 3 17.2 3 11.8 3 8 5.9 5.2 9.4 5.2c2.6 0 5 1.5 6.6 3.9 1.6-2.4 4-3.9 6.6-3.9C26.1 5.2 29 8 29 11.8c0 5.4-4 9.8-13 16.7z" fill="url(#hg)" />
    </svg>
  );
};

/** Final card, written like a git commit. */
export const Love: React.FC = () => {
  const f = useCurrentFrame();
  const { fps } = useVideoConfig();
  const cmd = 'git commit -m "Made with love"';
  const typed = Math.max(0, Math.min(cmd.length, Math.floor((f - 52) / 1.7)));
  const showCaret = f % 30 < 18;
  const card = spring({ frame: f - 118, fps, config: { damping: 18, stiffness: 90 } });
  const o = (a: number, b: number) => interpolate(f, [a, b], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp", easing: ease });
  return (
    <AbsoluteFill style={{ alignItems: "center", justifyContent: "center" }}>
      <div style={{ display: "flex", alignItems: "center", gap: 34, transform: `translateY(${-90 + (1 - o(0, 24)) * 20}px)` }}>
        <div style={{ opacity: o(0, 20) }}>
          <Heart size={150} />
        </div>
        <Kinetic text="Made with love." size={132} delay={6} />
      </div>

      <div style={{ position: "absolute", top: 560, display: "flex", flexDirection: "column", alignItems: "center", gap: 34 }}>
        <div
          style={{
            width: 1120,
            padding: "24px 34px",
            borderRadius: 16,
            background: "rgba(21,20,18,0.86)",
            border: `1px solid ${C.line}`,
            boxShadow: "0 30px 80px rgba(0,0,0,0.55)",
            fontFamily: "JB",
            fontSize: 38,
            color: C.text1,
            opacity: o(40, 56),
          }}
        >
          <span style={{ color: C.accent }}>$ </span>
          {cmd.slice(0, typed)}
          <span style={{ opacity: showCaret ? 1 : 0, color: C.accent }}>▌</span>
        </div>

        <div
          style={{
            width: 1120,
            padding: "30px 38px",
            borderRadius: 18,
            background: "linear-gradient(135deg, rgba(42,40,36,0.95), rgba(27,26,23,0.95))",
            border: `1px solid ${C.accent}55`,
            boxShadow: `0 40px 100px rgba(0,0,0,0.6), 0 0 90px rgba(229,138,85,0.18)`,
            display: "flex",
            alignItems: "center",
            gap: 30,
            opacity: card,
            transform: `translateY(${(1 - card) * 60}px) scale(${0.96 + card * 0.04})`,
          }}
        >
          <div style={{ width: 96, height: 96, borderRadius: 48, background: "linear-gradient(135deg,#F0A173,#B9683A)", display: "flex", alignItems: "center", justifyContent: "center", fontFamily: "Inter", fontWeight: 600, fontSize: 40, color: "#1A1109", boxShadow: "0 0 40px rgba(229,138,85,0.4)" }}>
            KY
          </div>
          <div style={{ flex: 1 }}>
            <div style={{ fontFamily: "Inter", fontWeight: 600, fontSize: 56, letterSpacing: -1, color: C.text1 }}>Kerem Berke Yılmaz</div>
            <div style={{ fontFamily: "JB", fontSize: 24, color: C.text3, marginTop: 8 }}>author · created Ferro</div>
          </div>
          <div style={{ textAlign: "right" }}>
            <div style={{ display: "inline-block", fontFamily: "JB", fontWeight: 700, fontSize: 24, padding: "6px 16px", borderRadius: 10, background: C.bg3, color: C.text2 }}>7f3a9c2</div>
            <div style={{ fontFamily: "Inter", fontSize: 22, color: C.green, marginTop: 12 }}>✓ committed just now</div>
          </div>
        </div>
      </div>
      <div style={{ position: "absolute", bottom: 40, fontFamily: "Inter", fontSize: 19, color: C.text3, opacity: o(150, 190) }}>
        Ferro is an independent concept project and is not affiliated with GitHub, Inc.
      </div>
    </AbsoluteFill>
  );
};
