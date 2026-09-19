"use client";

interface CrabMascotProps {
  size?: "sm" | "md" | "lg";
  mood?: "idle" | "anxious" | "happy" | "cheer" | "peek";
  className?: string;
}

const moodAnimations: Record<string, string> = {
  idle: "animate-crab-idle",
  anxious: "animate-claw-tap",
  happy: "animate-crab-happy",
  cheer: "animate-crab-cheer",
  peek: "animate-crab-reveal",
};

export function CrabMascot({ size = "md", mood = "idle", className = "" }: CrabMascotProps) {
  const sizeMap = { sm: 48, md: 80, lg: 120 };
  const s = sizeMap[size];
  const animClass = moodAnimations[mood] || moodAnimations.idle;

  return (
    <div className={`relative inline-block ${animClass} ${className}`}>
      <svg width={s} height={s * 0.75} viewBox="0 0 120 90" fill="none" xmlns="http://www.w3.org/2000/svg">
        {/* Body */}
        <ellipse cx="60" cy="55" rx="35" ry="25" fill="#E74C3C" />
        <ellipse cx="60" cy="55" rx="35" ry="25" fill="url(#crab-gradient)" />
        <ellipse cx="60" cy="50" rx="25" ry="15" fill="#C0392B" opacity="0.3" />
        {/* Eyes */}
        <circle cx="48" cy="35" r="8" fill="white" />
        <circle cx="72" cy="35" r="8" fill="white" />
        <circle cx="50" cy="34" r="4" fill="#10131a" />
        <circle cx="74" cy="34" r="4" fill="#10131a" />
        <circle cx="51" cy="33" r="1.5" fill="white" />
        <circle cx="75" cy="33" r="1.5" fill="white" />
        {/* Eye stalks */}
        <rect x="46" y="25" width="4" height="12" rx="2" fill="#E74C3C" />
        <rect x="70" y="25" width="4" height="12" rx="2" fill="#E74C3C" />
        {/* Claws */}
        <g>
          <ellipse cx="18" cy="48" rx="12" ry="8" fill="#E74C3C" />
          <ellipse cx="10" cy="45" rx="6" ry="4" fill="#E74C3C" />
          <ellipse cx="10" cy="51" rx="6" ry="4" fill="#E74C3C" />
        </g>
        <g>
          <ellipse cx="102" cy="48" rx="12" ry="8" fill="#E74C3C" />
          <ellipse cx="110" cy="45" rx="6" ry="4" fill="#E74C3C" />
          <ellipse cx="110" cy="51" rx="6" ry="4" fill="#E74C3C" />
        </g>
        {/* Legs */}
        <line x1="35" y1="70" x2="25" y2="82" stroke="#E74C3C" strokeWidth="3" strokeLinecap="round" />
        <line x1="45" y1="73" x2="38" y2="85" stroke="#E74C3C" strokeWidth="3" strokeLinecap="round" />
        <line x1="75" y1="73" x2="82" y2="85" stroke="#E74C3C" strokeWidth="3" strokeLinecap="round" />
        <line x1="85" y1="70" x2="95" y2="82" stroke="#E74C3C" strokeWidth="3" strokeLinecap="round" />
        {/* Mouth */}
        <path d="M52 60 Q60 65 68 60" stroke="#C0392B" strokeWidth="2" fill="none" strokeLinecap="round" />
        <defs>
          <linearGradient id="crab-gradient" x1="25" y1="30" x2="95" y2="80">
            <stop offset="0%" stopColor="#E74C3C" />
            <stop offset="100%" stopColor="#C0392B" />
          </linearGradient>
        </defs>
      </svg>
    </div>
  );
}
