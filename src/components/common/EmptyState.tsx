import type { ComponentType, ReactNode } from "react";
import type { IconProps } from "@phosphor-icons/react";
import { Icon } from "./Icon";

interface EmptyStateProps {
  icon: ComponentType<IconProps>;
  title: string;
  hint?: string;
  action?: ReactNode;
  error?: boolean;
}

export function EmptyState({
  icon,
  title,
  hint,
  action,
  error = false,
}: EmptyStateProps) {
  return (
    <div className={`empty-state${error ? " empty-state--error" : ""}`}>
      <Icon icon={icon} size={40} />
      <p className="empty-state__title">{title}</p>
      {hint && <p className="empty-state__hint">{hint}</p>}
      {action && <div className="empty-state__action">{action}</div>}
    </div>
  );
}
