import type { ButtonHTMLAttributes, ComponentType } from "react";
import type { IconProps } from "@phosphor-icons/react";
import { Icon } from "./Icon";

export interface IconButtonProps
  extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, "children"> {
  icon: ComponentType<IconProps>;
  label: string;
  iconSize?: number;
  weight?: IconProps["weight"];
}

/**
 * 图标按钮：最小命中区域 40×32px（≥24px），带 aria-label 与可见焦点环。
 */
export function IconButton({
  icon,
  label,
  iconSize = 18,
  weight,
  className = "",
  type = "button",
  ...rest
}: IconButtonProps) {
  return (
    <button
      type={type}
      aria-label={label}
      title={label}
      className={`icon-button ${className}`.trim()}
      {...rest}
    >
      <Icon icon={icon} size={iconSize} weight={weight} />
    </button>
  );
}
