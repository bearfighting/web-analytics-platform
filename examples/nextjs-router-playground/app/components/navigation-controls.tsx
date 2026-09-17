"use client";

import Link from "next/link";
import { usePathname, useRouter, useSearchParams } from "next/navigation";

export function NavigationControls() {
  const pathname = usePathname();
  const router = useRouter();
  const searchParams = useSearchParams();

  const updateSearchParams = () => {
    const params = new URLSearchParams(searchParams.toString());
    params.set("q", "router");
    router.push(`${pathname}?${params.toString()}`);
  };

  return (
    <nav aria-label="Navigation controls">
      <h2>Navigation Controls</h2>
      <ul>
        <li>
          <Link href="/">Home</Link>
        </li>
        <li>
          <Link href="/about">About via Link</Link>
        </li>
        <li>
          <Link href="/products/example">Dynamic product</Link>
        </li>
        <li>
          <Link href="/nested">Nested page</Link>
        </li>
        <li>
          <Link href="/nested/child">Nested child</Link>
        </li>
        <li>
          <Link href="/search?q=link">Search with query</Link>
        </li>
      </ul>
      <button type="button" onClick={() => router.push("/about?source=push")}>
        router.push()
      </button>{" "}
      <button type="button" onClick={() => router.replace("/about?source=replace")}>
        router.replace()
      </button>{" "}
      <button type="button" onClick={() => router.back()}>
        router.back()
      </button>{" "}
      <button type="button" onClick={() => router.forward()}>
        router.forward()
      </button>{" "}
      <button type="button" onClick={updateSearchParams}>
        Update search params
      </button>{" "}
      <a href="#details">Update hash</a>
      <p id="details">Hash navigation target.</p>
    </nav>
  );
}
