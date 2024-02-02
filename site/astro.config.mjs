import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";

// https://astro.build/config
export default defineConfig({
  integrations: [
    starlight({
      title: "Chisel Operator",
      social: {
        github: "https://github.com/fyralabs/chisel-operator",
        discord: "https://discord.com/invite/5fdPuxTg5Q",
        matrix: "https://matrix.to/#/#hub:fyralabs.com",
        twitter: "https://twitter.com/teamfyralabs",
        mastodon: "https://fedi.fyralabs.com/@hq",
      },
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
          label: "Cloud Provisioning",
          autogenerate: { directory: "cloud" },
        },
      ],
    }),
  ],
});
