import { createHashRouter, RouterProvider } from 'react-router';
import { AppShell } from '@/components/AppShell';
import { StoreProvider } from '@/lib/store';
import About from '@/pages/About';
import Device from '@/pages/Device';
import NotFound from '@/pages/NotFound';
import Overview from '@/pages/Overview';

// Hash routing, not browser routing. It keeps every route on index.html, so the local service needs
// no rewrite rules, and the tray can open a device directly at `#/device/<udid>`.
const router = createHashRouter([
  {
    element: <AppShell />,
    children: [
      { index: true, element: <Overview /> },
      { path: 'device/:udid', element: <Device /> },
      { path: 'about', element: <About /> },
      { path: '*', element: <NotFound /> },
    ],
  },
]);

export default function App() {
  return (
    <StoreProvider>
      <RouterProvider router={router} />
    </StoreProvider>
  );
}
