import React from "react";
import { AbsoluteFill, Img, OffthreadVideo, Sequence, interpolate, spring, staticFile, useCurrentFrame, useVideoConfig } from "remotion";
import { Love } from "./Love";
import { Background, C, Chip, Eyebrow, FMark, Key, Kinetic, Sub, Win, ease, fontCss, useFade } from "./ui";

// scene boundaries (30 fps)
const S = {
  intro: [0, 150],
  logo: [150, 318],
  hero: [318, 524],
  stage: [524, 988],
  palette: [988, 1349],
  history: [1349, 1555],
  undo: [1555, 1709],
  looks: [1709, 1864],
  numbers: [1864, 2019],
  table: [2019, 2173],
  setup: [2173, 2328],
  outro: [2328, 2483],
  love: [2483, 2740],
} as const;
export const TOTAL = 2740;

const Scene: React.FC<{ range: readonly [number, number]; children: React.ReactNode; bg?: number; slowIn?: boolean }> = ({ range, children, bg = 1, slowIn = false }) => {
  const dur = range[1] - range[0];
  return (
    <Sequence from={range[0]} durationInFrames={dur}>
      <Inner dur={dur} bg={bg} slowIn={slowIn}>
        {children}
      </Inner>
    </Sequence>
  );
};

const Inner: React.FC<{ dur: number; bg: number; slowIn: boolean; children: React.ReactNode }> = ({ dur, bg, slowIn, children }) => {
  const o = useFade(dur, slowIn ? 40 : 12, slowIn ? 18 : 12);
  return (
    <AbsoluteFill>
      {/* the very first scene fades up from black, background included */}
      <AbsoluteFill style={{ opacity: slowIn ? o : 1 }}>
        <Background intensity={bg} />
      </AbsoluteFill>
      <AbsoluteFill style={{ opacity: o }}>{children}</AbsoluteFill>
    </AbsoluteFill>
  );
};

const useIn = (delay = 0, len = 30) => {
  const f = useCurrentFrame();
  return interpolate(f - delay, [0, len], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp", easing: ease });
};

// ---------------------------------------------------------------- scenes

const Intro: React.FC = () => (
  <AbsoluteFill style={{ padding: "0 160px", justifyContent: "center" }}>
    <Kinetic text="Git is something you do all day." size={104} delay={16} />
    <div style={{ height: 36 }} />
    <Kinetic text="Your client should keep up." size={104} delay={74} color={C.accent} />
  </AbsoluteFill>
);

const Logo: React.FC = () => {
  const f = useCurrentFrame();
  const { fps } = useVideoConfig();
  const s = spring({ frame: f - 12, fps, config: { damping: 14, mass: 0.9, stiffness: 90 } });
  const ring = (k: number) => {
    const p = ((f - 20 - k * 22) % 90) / 90;
    return { opacity: f > 20 ? (1 - p) * 0.35 : 0, transform: `scale(${1 + p * 2.4})` };
  };
  return (
    <AbsoluteFill style={{ alignItems: "center", justifyContent: "center" }}>
      <div style={{ position: "relative", width: 220, height: 220, transform: `scale(${0.5 + s * 0.5}) rotate(${(1 - s) * -14}deg)`, opacity: s }}>
        {[0, 1, 2].map((k) => (
          <div key={k} style={{ position: "absolute", inset: 0, borderRadius: 56, border: `3px solid ${C.accent}`, ...ring(k) }} />
        ))}
        <FMark size={220} />
      </div>
      <div style={{ marginTop: 56 }}>
        <Kinetic text="Ferro" size={170} delay={26} align="center" />
      </div>
      <Sub delay={52} size={36} width={1200}>
        <span style={{ display: "block", textAlign: "center" }}>A native Git client — drawn by hand, in Rust.</span>
      </Sub>
    </AbsoluteFill>
  );
};

const Hero: React.FC = () => {
  const f = useCurrentFrame();
  const { fps } = useVideoConfig();
  const sp = spring({ frame: f - 8, fps, config: { damping: 20, mass: 1, stiffness: 70 } });
  const push = interpolate(f, [0, 240], [1, 1.07]);
  return (
    <AbsoluteFill>
      <div style={{ position: "absolute", left: 130, top: 120 }}>
        <Eyebrow delay={4}>Meet Ferro</Eyebrow>
        <Kinetic text="Fast. Calm." size={110} delay={8} />
        <Kinetic text="Entirely native." size={110} delay={22} color={C.accent} />
      </div>
      <div style={{ position: "absolute", left: 930, top: 440, transform: `translateY(${(1 - sp) * 220}px) scale(${push})`, transformOrigin: "0 0", opacity: sp }}>
        <Win width={1560}>
          <Img src={staticFile("img/c2.png")} style={{ width: "100%", height: "100%", objectFit: "cover", objectPosition: "top left" }} />
        </Win>
      </div>
      <div style={{ position: "absolute", left: 130, top: 600, display: "flex", flexDirection: "column", gap: 22, opacity: useIn(70) }}>
        <Chip>Rust. No web view, no Electron.</Chip>
        <Chip color={C.blue}>Hunk and line staging</Chip>
        <Chip color={C.green}>Opens in a blink</Chip>
      </div>
    </AbsoluteFill>
  );
};

const Caption: React.FC<{ from: number; to: number; title: string; keys?: string[]; sub?: string }> = ({ from, to, title, keys, sub }) => {
  const f = useCurrentFrame();
  const o = interpolate(f, [from, from + 10, to - 10, to], [0, 1, 1, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
  const y = interpolate(f, [from, from + 14], [24, 0], { extrapolateLeft: "clamp", extrapolateRight: "clamp", easing: ease });
  return (
    <div style={{ position: "absolute", left: 0, right: 0, top: 70, display: "flex", justifyContent: "center", opacity: o, transform: `translateY(${y}px)` }}>
      <div style={{ textAlign: "center" }}>
        <div style={{ fontFamily: "Inter", fontWeight: 600, fontSize: 64, letterSpacing: -1.5, color: C.text1 }}>
          {title}
          {keys?.map((k) => (
            <Key key={k}>{k}</Key>
          ))}
        </div>
        {sub && <div style={{ fontFamily: "Inter", fontSize: 28, color: C.text2, marginTop: 10 }}>{sub}</div>}
      </div>
    </div>
  );
};

const ClipScene: React.FC<{ src: string; startSec: number; rate?: number; width?: number; top?: number }> = ({ src, startSec, rate = 1, width = 1560, top = 215 }) => {
  const f = useCurrentFrame();
  const { fps } = useVideoConfig();
  const sp = spring({ frame: f, fps, config: { damping: 22, stiffness: 80 } });
  return (
    <div style={{ position: "absolute", left: (1920 - width) / 2, top, transform: `scale(${0.94 + sp * 0.06})`, opacity: sp, transformOrigin: "50% 0" }}>
      <Win width={width}>
        <OffthreadVideo src={staticFile(src)} startFrom={Math.round(startSec * fps)} playbackRate={rate} muted style={{ width: "100%", height: "100%", objectFit: "cover" }} />
      </Win>
    </div>
  );
};

const Stage: React.FC = () => (
  <AbsoluteFill>
    <ClipScene src="clips/clipA.mp4" startSec={1.4} rate={1.22} />
    <Caption from={4} to={118} title="Stage by line." sub="Click, shift-click, done. Hunks work too." />
    <Caption from={122} to={268} title="Write it, commit it." keys={["Ctrl", "↵"]} />
    <Caption from={272} to={446} title="Changed your mind?" keys={["Ctrl", "Z"]} sub="Ferro snapshots every write, so undo always works." />
  </AbsoluteFill>
);

const Palette: React.FC = () => (
  <AbsoluteFill>
    <ClipScene src="clips/clipB.mp4" startSec={1.3} rate={1.0} />
    <Caption from={4} to={160} title="Everything is one shortcut away." keys={["Ctrl", "K"]} />
    <Caption from={168} to={326} title="Switch branches, keep your work." keys={["Ctrl", "B"]} sub="Stash it, or bring your changes along." />
  </AbsoluteFill>
);

const History: React.FC = () => (
  <AbsoluteFill>
    <ClipScene src="clips/clipB.mp4" startSec={12.2} rate={1.0} />
    <Caption from={4} to={236} title="A real commit graph." sub="Merges, branches and tags — finally visible." />
  </AbsoluteFill>
);

const Undo: React.FC = () => {
  const f = useCurrentFrame();
  const { fps } = useVideoConfig();
  const sp = spring({ frame: f, fps, config: { damping: 24, stiffness: 70 } });
  const z = interpolate(f, [0, 180], [1, 1.12]);
  return (
    <AbsoluteFill>
      <div style={{ position: "absolute", left: 130, top: 150 }}>
        <Eyebrow>Operation log</Eyebrow>
        <div style={{ width: 800 }}><Kinetic text="Every action is undoable." size={92} delay={4} /></div>
        <Sub delay={30} size={32} width={760}>
          Stage, commit, switch, pull, merge. Ferro keeps a snapshot of each one — jump back to any point, or step forward again.
        </Sub>
        <div style={{ marginTop: 34, opacity: useIn(50) }}>
          <Key>Ctrl</Key>
          <Key>Z</Key>
          <span style={{ fontFamily: "Inter", color: C.text3, fontSize: 28, margin: "0 18px" }}>undo</span>
          <Key>Ctrl</Key>
          <Key>⇧</Key>
          <Key>Z</Key>
          <span style={{ fontFamily: "Inter", color: C.text3, fontSize: 28, marginLeft: 18 }}>redo</span>
        </div>
      </div>
      <div style={{ position: "absolute", left: 1010, top: 330, transform: `translateX(${(1 - sp) * 200}px) scale(${z})`, transformOrigin: "0 0", opacity: sp }}>
        <Win width={1560}>
          <Img src={staticFile("img/oplog.png")} style={{ width: "100%", height: "100%", objectFit: "cover", objectPosition: "top left" }} />
        </Win>
      </div>
    </AbsoluteFill>
  );
};

const Looks: React.FC = () => {
  const f = useCurrentFrame();
  const { fps } = useVideoConfig();
  const a = spring({ frame: f - 4, fps, config: { damping: 20, stiffness: 70 } });
  const b = spring({ frame: f - 16, fps, config: { damping: 20, stiffness: 70 } });
  return (
    <AbsoluteFill>
      <div style={{ position: "absolute", left: 0, right: 0, top: 90, textAlign: "center" }}>
        <Eyebrow>Made for you</Eyebrow>
        <div style={{ display: "flex", justifyContent: "center" }}>
          <Kinetic text="Light. Dark. English. Türkçe." size={92} delay={4} align="center" />
        </div>
        <div style={{ display: "flex", justifyContent: "center" }}>
          <Sub delay={34} size={30} width={1200}>
            <span style={{ display: "block", textAlign: "center" }}>Ferro follows your system language and theme — and every setting is one click away.</span>
          </Sub>
        </div>
      </div>
      <div style={{ position: "absolute", left: 110, top: 470, transform: `translateY(${(1 - a) * 260}px) rotate(${-2.2}deg)`, opacity: a }}>
        <Win width={880} glow={false}>
          <Img src={staticFile("img/light.png")} style={{ width: "100%", height: "100%", objectFit: "cover" }} />
        </Win>
      </div>
      <div style={{ position: "absolute", left: 960, top: 500, transform: `translateY(${(1 - b) * 260}px) rotate(${1.6}deg)`, opacity: b }}>
        <Win width={880} glow={false}>
          <Img src={staticFile("img/turkish.png")} style={{ width: "100%", height: "100%", objectFit: "cover" }} />
        </Win>
      </div>
    </AbsoluteFill>
  );
};

type Metric = { label: string; unit: string; ferro: number; gh: number; fmt: (v: number) => string; note: string };
const METRICS: Metric[] = [
  { label: "Memory, idle", unit: "", ferro: 110, gh: 415, fmt: (v) => `${Math.round(v)} MB`, note: "1 process vs 4 processes" },
  { label: "On disk", unit: "", ferro: 7.6, gh: 719, fmt: (v) => (v < 100 ? `${v.toFixed(1)} MB` : `${Math.round(v)} MB`), note: "one installed version" },
  { label: "First window", unit: "", ferro: 0.3, gh: 0.9, fmt: (v) => `${v.toFixed(1)} s`, note: "average of two cold starts" },
];

const Numbers: React.FC = () => {
  const f = useCurrentFrame();
  return (
    <AbsoluteFill style={{ padding: "100px 150px" }}>
      <Eyebrow>Measured, not promised</Eyebrow>
      <Kinetic text="Light on your machine." size={100} delay={2} />
      <div style={{ marginTop: 70, display: "flex", flexDirection: "column", gap: 54 }}>
        {METRICS.map((m, i) => {
          const t = interpolate(f, [26 + i * 22, 26 + i * 22 + 46], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp", easing: ease });
          const max = Math.max(m.ferro, m.gh);
          const W = 1150;
          return (
            <div key={m.label} style={{ opacity: Math.min(1, t * 3) }}>
              <div style={{ display: "flex", justifyContent: "space-between", fontFamily: "Inter", fontSize: 28, color: C.text2, marginBottom: 16, width: W + 260 }}>
                <span style={{ fontWeight: 600, color: C.text1 }}>{m.label}</span>
                <span style={{ color: C.text3 }}>{m.note}</span>
              </div>
              {[
                { name: "Ferro", v: m.ferro, col: C.accent, txt: C.accentHi },
                { name: "GitHub Desktop", v: m.gh, col: "#5A564E", txt: C.text2 },
              ].map((r) => (
                <div key={r.name} style={{ display: "flex", alignItems: "center", gap: 24, marginBottom: 12 }}>
                  <div style={{ width: 230, fontFamily: "Inter", fontSize: 24, color: r.txt, fontWeight: 600 }}>{r.name}</div>
                  <div style={{ width: W, height: 26, borderRadius: 13, background: "rgba(255,255,255,0.05)", overflow: "hidden" }}>
                    <div style={{ width: Math.max(10, (r.v / max) * W * t), height: "100%", borderRadius: 13, background: r.col, boxShadow: r.name === "Ferro" ? `0 0 26px ${C.accent}88` : "none" }} />
                  </div>
                  <div style={{ fontFamily: "JB", fontWeight: 700, fontSize: 30, color: r.txt, width: 150 }}>{m.fmt(r.v * t)}</div>
                </div>
              ))}
            </div>
          );
        })}
      </div>
      <div style={{ position: "absolute", left: 150, bottom: 52, fontFamily: "Inter", fontSize: 21, color: C.text3 }}>
        Measured on one Windows 11 PC · GitHub Desktop 3.6.4 · idle for 14 s · your results will vary
      </div>
    </AbsoluteFill>
  );
};

const ROWS: [string, string][] = [
  ["A commit graph in History", "No graph"],
  ["Undo any operation, with a full operation log", "Undo last commit only"],
  ["Command palette and branch switcher", "No palette"],
  ["Every stash, browsable with diffs", "One stash per branch"],
  ["The exact git command, always visible", "Hidden"],
  ["Follows your system language and theme", "English only"],
];

const Table: React.FC = () => {
  const f = useCurrentFrame();
  return (
    <AbsoluteFill style={{ padding: "100px 150px" }}>
      <Eyebrow>Where it pulls ahead</Eyebrow>
      <Kinetic text="Built for the way you work." size={96} delay={2} />
      <div style={{ marginTop: 56, width: 1560 }}>
        <div style={{ display: "flex", fontFamily: "Inter", fontSize: 24, color: C.text3, paddingBottom: 14, borderBottom: `1px solid ${C.line}` }}>
          <div style={{ flex: 1 }} />
          <div style={{ width: 470, color: C.accentHi, fontWeight: 600 }}>Ferro</div>
          <div style={{ width: 330, fontWeight: 600 }}>GitHub Desktop</div>
        </div>
        {ROWS.map(([a, b], i) => {
          const t = interpolate(f, [16 + i * 12, 16 + i * 12 + 22], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp", easing: ease });
          return (
            <div key={a} style={{ display: "flex", alignItems: "center", height: 84, borderBottom: `1px solid ${C.line}`, opacity: t, transform: `translateX(${(1 - t) * 40}px)`, fontFamily: "Inter", fontSize: 30 }}>
              <div style={{ flex: 1, color: C.text1 }}>{a}</div>
              <div style={{ width: 470, color: C.green, fontWeight: 600, fontSize: 40 }}>✓</div>
              <div style={{ width: 330, color: C.text3, fontSize: 26 }}>{b}</div>
            </div>
          );
        })}
      </div>
    </AbsoluteFill>
  );
};

const Setup: React.FC = () => {
  const f = useCurrentFrame();
  const { fps } = useVideoConfig();
  const imgs = ["img/inst_custom.png", "img/inst_progress.png", "img/inst_done.png"];
  return (
    <AbsoluteFill>
      <div style={{ position: "absolute", left: 0, right: 0, top: 100, textAlign: "center" }}>
        <Eyebrow>Setup</Eyebrow>
        <div style={{ display: "flex", justifyContent: "center" }}>
          <Kinetic text="A 6 MB installer." size={96} delay={2} align="center" />
        </div>
        <div style={{ display: "flex", justifyContent: "center" }}>
          <Sub delay={26} size={32} width={1300}>
            <span style={{ display: "block", textAlign: "center" }}>No admin rights. One click. Removes itself cleanly.</span>
          </Sub>
        </div>
      </div>
      {imgs.map((src, i) => {
        const s = spring({ frame: f - 10 - i * 8, fps, config: { damping: 20, stiffness: 70 } });
        const x = 130 + i * 590;
        return (
          <div key={src} style={{ position: "absolute", left: x, top: 470 + (i === 1 ? -26 : 0), transform: `translateY(${(1 - s) * 240}px) rotate(${(i - 1) * 1.6}deg)`, opacity: s }}>
            <div style={{ width: 560, height: 560 * (540 / 880), borderRadius: 22, overflow: "hidden", boxShadow: "0 0 0 1px rgba(255,255,255,0.09), 0 40px 90px rgba(0,0,0,0.6), 0 0 120px rgba(229,138,85,0.12)" }}>
              <Img src={staticFile(src)} style={{ width: "100%", height: "100%" }} />
            </div>
          </div>
        );
      })}
    </AbsoluteFill>
  );
};

const Outro: React.FC = () => {
  const f = useCurrentFrame();
  const { fps } = useVideoConfig();
  const s = spring({ frame: f - 4, fps, config: { damping: 16, stiffness: 80 } });
  return (
    <AbsoluteFill style={{ alignItems: "center", justifyContent: "center" }}>
      <div style={{ transform: `scale(${0.7 + s * 0.3})`, opacity: s }}>
        <FMark size={150} />
      </div>
      <div style={{ marginTop: 44 }}>
        <Kinetic text="Git, without the weight." size={112} delay={18} align="center" />
      </div>
      <Sub delay={54} size={32} width={1400}>
        <span style={{ display: "block", textAlign: "center" }}>Ferro · built in Rust · hand-drawn UI · English &amp; Türkçe</span>
      </Sub>
    </AbsoluteFill>
  );
};

export const Promo: React.FC = () => (
  <AbsoluteFill style={{ background: C.bg0 }}>
    <style>{fontCss}</style>
    <Scene range={S.intro} bg={0.6} slowIn>
      <Intro />
    </Scene>
    <Scene range={S.logo}>
      <Logo />
    </Scene>
    <Scene range={S.hero}>
      <Hero />
    </Scene>
    <Scene range={S.stage} bg={0.5}>
      <Stage />
    </Scene>
    <Scene range={S.palette} bg={0.5}>
      <Palette />
    </Scene>
    <Scene range={S.history} bg={0.5}>
      <History />
    </Scene>
    <Scene range={S.undo}>
      <Undo />
    </Scene>
    <Scene range={S.looks}>
      <Looks />
    </Scene>
    <Scene range={S.numbers} bg={0.7}>
      <Numbers />
    </Scene>
    <Scene range={S.table} bg={0.7}>
      <Table />
    </Scene>
    <Scene range={S.setup}>
      <Setup />
    </Scene>
    <Scene range={S.outro}>
      <Outro />
    </Scene>
    <Scene range={S.love} bg={0.9}>
      <Love />
    </Scene>
  </AbsoluteFill>
);
