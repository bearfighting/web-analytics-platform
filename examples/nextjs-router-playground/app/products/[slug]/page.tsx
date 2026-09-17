export default async function ProductPage({ params }: { params: Promise<{ slug: string }> }) {
  const { slug } = await params;

  return (
    <main>
      <h1>Product: {slug}</h1>
      <p>This page is used to test a dynamic App Router segment.</p>
    </main>
  );
}
