import { useToastStore } from "../../stores/toastStore";

export function Toaster() {
  const message = useToastStore((state) => state.message);
  if (!message) return null;
  return (
    <div className="toaster" role="status" aria-live="polite">
      {message}
    </div>
  );
}
