// The landing page below the top: how it works, what it handles, and a last
// way in. Each is a slide (`Slides.tsx`): what carries `data-rise` rises in as
// the slide fades in, staggered by `kit.stagger`.

import { Clapperboard, FolderUp, MessageSquareText } from "lucide-react";
import { useT } from "@/i18n/I18nProvider";
import { Feature } from "./Feature";
import { Section } from "./Section";

export function HowItWorks() {
  const t = useT().landing.how;
  const steps = [
    { icon: FolderUp, ...t.files },
    { icon: MessageSquareText, ...t.describe },
    { icon: Clapperboard, ...t.adjust },
  ];
  return (
    <Section id="how" eyebrow={t.eyebrow} title={t.title}>
      <div className="mt-12 grid gap-4 md:grid-cols-3">
        {steps.map((step, index) => (
          <Feature key={step.title} {...step} index={index} step={`0${index + 1}`} />
        ))}
      </div>
    </Section>
  );
}
