import { useEffect } from 'react';

const SITE_NAME = 'iPhone Inspector';

export function useHead({ title, description }: { title?: string; description?: string }) {
  useEffect(() => {
    const prev = document.title;
    document.title = title ? `${title} | ${SITE_NAME}` : SITE_NAME;
    return () => {
      document.title = prev;
    };
  }, [title]);

  useEffect(() => {
    const tag = document.querySelector('meta[name="description"]');
    if (!tag || !description) return;
    const prev = tag.getAttribute('content') ?? '';
    tag.setAttribute('content', description);
    return () => {
      tag.setAttribute('content', prev);
    };
  }, [description]);
}
