"use client";

import Image from "next/image";

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
  peek: "animate-crab-peek",
};

const sizeMap = { sm: 64, md: 96, lg: 160 } as const;

export function CrabMascot({
  size = "md",
  mood = "idle",
  className = "",
}: CrabMascotProps) {
  const width = sizeMap[size];
  const height = Math.round((width * 991) / 1338);
  const animClass = moodAnimations[mood] || moodAnimations.idle;

  return (
    <div
      className={`relative inline-block ${animClass} ${className}`}
      style={{ width, height }}
    >
      <Image
        src="/mascot.png"
        alt="StellarCard crab mascot"
        width={width}
        height={height}
        className="h-auto w-full select-none drop-shadow-lg"
        draggable={false}
      />
    </div>
  );
}
