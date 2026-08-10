import { useEffect, useRef, useState } from "react";

interface ScrollingTextProps {
  text: string;
  className?: string;
}

/** 超长文本自动折叠为跑马灯滑动展示；未溢出时保持省略号。 */
export function ScrollingText({ text, className = "" }: ScrollingTextProps) {
  const outerRef = useRef<HTMLSpanElement>(null);
  const [scrolling, setScrolling] = useState(false);

  useEffect(() => {
    const outer = outerRef.current;
    if (!outer) return;
    const check = () => {
      setScrolling(outer.scrollWidth > outer.clientWidth + 2);
    };
    check();
    const observer = new ResizeObserver(check);
    observer.observe(outer);
    return () => observer.disconnect();
  }, [text]);

  if (!scrolling) {
    return (
      <span ref={outerRef} className={`${className} scrolling-text`} title={text}>
        {text}
      </span>
    );
  }

  const duration = Math.max(6, Math.round(text.length * 0.18));
  return (
    <span
      ref={outerRef}
      className={`${className} scrolling-text scrolling-text--active`}
      title={text}
    >
      <span
        className="scrolling-text__track"
        style={{ animationDuration: `${duration}s` }}
      >
        <span className="scrolling-text__content">{text}</span>
        <span className="scrolling-text__content" aria-hidden>
          {text}
        </span>
      </span>
    </span>
  );
}
