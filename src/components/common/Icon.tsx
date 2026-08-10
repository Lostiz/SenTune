import type { ComponentType } from "react";
import type { IconProps } from "@phosphor-icons/react";

export interface AppIconProps {
  icon: ComponentType<IconProps>;
  size?: number;
  weight?: IconProps["weight"];
  className?: string;
  ariaHidden?: boolean;
}

/**
 * 统一图标入口：默认跟随全局 IconContext（weight="light"、currentColor）。
 */
export function Icon({
  icon: Glyph,
  size = 20,
  weight,
  className,
  ariaHidden = true,
}: AppIconProps) {
  return (
    <Glyph
      size={size}
      weight={weight}
      className={className}
      aria-hidden={ariaHidden}
    />
  );
}
