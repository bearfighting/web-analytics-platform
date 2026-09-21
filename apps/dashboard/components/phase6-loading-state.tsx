import React from "react";

export function Phase6LoadingState({ heading }: { heading: string }) {
  return (
    <section className="card" aria-labelledby={`${heading}-loading-heading`}>
      <h2 id={`${heading}-loading-heading`}>{heading}</h2>
      <p role="status">Loading Phase 6 analytics...</p>
    </section>
  );
}
