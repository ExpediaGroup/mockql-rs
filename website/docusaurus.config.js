module.exports = {
  title: 'MockQL Rust',
  tagline: 'Directive-driven GraphQL response mocking via @mock',
  url: 'https://expediagroup.github.io',
  baseUrl: '/mockql-rs/',
  organizationName: 'ExpediaGroup',
  projectName: 'mockql-rs',
  customFields: {
    repoUrl: 'https://github.com/ExpediaGroup/mockql-rs'
  },
  markdown: {
    mermaid: true,
    hooks: {
      onBrokenMarkdownLinks: 'throw'
    }
  },
  onBrokenLinks: 'throw',
  onDuplicateRoutes: 'throw',
  presets: [
    [
      '@docusaurus/preset-classic',
      {
        blog: false,
        docs: {
          editUrl: 'https://github.com/ExpediaGroup/mockql-rs/tree/main/website',
          showLastUpdateAuthor: true,
          showLastUpdateTime: true,
          sidebarPath: require.resolve('./sidebars.js')
        }
      }
    ]
  ],
  themes: ['@docusaurus/theme-mermaid'],
  themeConfig: {
    colorMode: {
      defaultMode: 'dark',
    },
    prism: {
      additionalLanguages: ['bash'],
      theme: require('prism-react-renderer').themes.github,
      darkTheme: require('prism-react-renderer').themes.dracula
    },
    navbar: {
      title: 'MockQL Rust',
      logo: {
        src: 'img/EG_Icon_White_on_Blue.png',
        href: '/docs'
      },
      items: [
        {
          type: 'docSidebar',
          sidebarId: 'docs',
          label: 'Docs',
          position: 'right'
        },
        {
          label: 'Commands',
          position: 'right',
          items: [
            {
              type: 'doc',
              docId: 'commands/oneshot',
              label: 'oneshot'
            },
            {
              type: 'doc',
              docId: 'commands/proxy',
              label: 'proxy'
            },
            {
              type: 'doc',
              docId: 'commands/schema',
              label: 'schema'
            }
          ]
        },
        {
          label: 'Providers',
          position: 'right',
          items: [
            {
              type: 'doc',
              docId: 'provider-cli',
              label: 'CLI Providers'
            },
            {
              type: 'doc',
              docId: 'provider-http',
              label: 'HTTP Providers'
            }
          ]
        },
        {
          href: 'https://github.com/ExpediaGroup/mockql-rs',
          label: 'GitHub',
          position: 'right'
        }
      ]
    },
    footer: {
      links: [],
      copyright: 'Copyright © 2026 Expedia, Inc.',
      logo: {
        src: 'img/Expedia-Group-Logo_E-Stacked.png'
      }
    }
  }
};
