export default function NestedLayout({ children }: Readonly<{ children: React.ReactNode }>) {
  return (
    <section>
      <p>Shared nested layout.</p>
      {children}
    </section>
  );
}
