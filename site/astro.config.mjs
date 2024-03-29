import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";

// https://astro.build/config
export default defineConfig({
  site: "https://chisel.fyralabs.com",
  integrations: [
    starlight({
      title: "Chisel Operator",
      editLink: {
        baseUrl: "https://github.com/FyraLabs/chisel-operator/edit/main/site/",
      },
      social: {
        github: "https://github.com/fyralabs/chisel-operator",
        discord: "https://discord.com/invite/5fdPuxTg5Q",
        matrix: "https://matrix.to/#/#hub:fyralabs.com",
        twitter: "https://twitter.com/teamfyralabs",
        mastodon: "https://fedi.fyralabs.com/@hq",
      },
      head: [
        {
          tag: "script",
          attrs: {
            src: "https://plausible.fyralabs.com/js/script.js",
            "data-domain": "chisel.fyralabs.com",
            defer: true,
          },
        },
      ],
      sidebar: [
        {
          label: "Guides",
          items: [
            { label: "Installation", link: "/guides/installation/" },
            {
              label: "Exposing a Service",
              link: "/guides/exposing-a-service/",
            },
            {
              label: "Self Hosting an Exit Node",
              link: "/guides/self-host-exit-node/",
            },
            {
              label: "Using Cloud Provisioning",
              link: "/guides/using-cloud-provisioning/",
            },
          ],
        },
        {
          label: "Reference",
          autogenerate: { directory: "reference" },
        },
        {
          label: "Cloud Provisioning Reference",
          autogenerate: { directory: "cloud" },
        },
      ],
    }),
  ],
});
