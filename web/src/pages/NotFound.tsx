import { Link } from 'react-router';
import { useHead } from '@/lib/use-head';

export default function NotFound() {
  useHead({ title: 'Not Found' });
  return (
    <div className="py-20 text-center">
      <h1 className="text-xl font-extrabold tracking-tight">Page Not Found</h1>
      <p className="mt-2 text-base text-slate-600 dark:text-slate-400">That route does not exist.</p>
      <Link to="/" className="mt-5 inline-block text-base font-semibold text-blue-600 hover:underline dark:text-blue-400">
        Back to All iPhones
      </Link>
    </div>
  );
}
