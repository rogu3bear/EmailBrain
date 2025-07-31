import './globals.css';
import type { ReactNode } from 'react';

export const metadata = {
  title: 'EmailBrain',
  description: 'Email analysis made easy',
};

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="en" suppressHydrationWarning>
      <body className="min-h-screen bg-gray-50 text-gray-900 dark:bg-gray-900 dark:text-gray-100 antialiased">
        {children}
      </body>
    </html>
  );
}
