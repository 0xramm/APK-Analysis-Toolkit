// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

// https://astro.build/config
export default defineConfig({
	site: 'https://0xramm.github.io',
	base: '/APK-Analysis-Toolkit',
	integrations: [
		starlight({
			title: 'APK Analysis Toolkit',
			description:
				'A Rust TUI that bootstraps a rooted Android sandbox, SSL-pinning bypass, mitmproxy interception, and automated malware-style package analysis.',
			social: [{ icon: 'github', label: 'GitHub', href: 'https://github.com/0xramm/APK-Analysis-Toolkit' }],
			sidebar: [
				{
					label: 'Getting Started',
					items: [
						{ label: 'Overview', slug: 'getting-started/overview' },
						{ label: 'Installation', slug: 'getting-started/installation' },
						{ label: 'Quick Start', slug: 'getting-started/quick-start' },
					],
				},
				{
					label: 'Emulator',
					items: [
						{ label: 'AVDs & Rooting', slug: 'emulator/avds-and-rooting' },
					],
				},
				{
					label: 'SSL Pinning Bypass',
					items: [
						{ label: 'Bypass & Re-pin', slug: 'ssl-pinning/bypass-and-repin' },
					],
				},
				{
					label: 'mitmproxy',
					items: [
						{ label: 'Proxy & CA Certificate', slug: 'mitmproxy/proxy-and-cert' },
					],
				},
				{
					label: 'App Manager',
					items: [
						{ label: 'Listing & Filtering', slug: 'app-manager/listing-and-filtering' },
						{ label: 'Package Info & Red Flags', slug: 'app-manager/package-info-and-red-flags' },
					],
				},
				{
					label: 'Operations',
					items: [
						{ label: 'Multi-Device Selection', slug: 'operations/multi-device' },
						{ label: 'Full Setup Pipeline', slug: 'operations/full-setup' },
						{ label: 'Auto-Update & Releases', slug: 'operations/auto-update' },
					],
				},
				{
					label: 'Troubleshooting',
					items: [{ label: 'Common Issues', slug: 'troubleshooting/common-issues' }],
				},
			],
		}),
	],
});
