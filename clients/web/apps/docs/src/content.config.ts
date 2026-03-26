import { defineCollection } from "astro:content";
import { docsLoader } from "@astrojs/starlight/loaders";
import { docsSchema } from "@astrojs/starlight/schema";
import { z } from "astro/zod";

export const collections = {
  docs: defineCollection({
    loader: docsLoader(),
    schema: docsSchema({
      extend: z.object({
        owner: z.string().min(3).optional(),
        status: z.enum(["canonical", "draft", "deprecated"]).optional(),
        lastReviewed: z.coerce.date().optional(),
        appliesTo: z.string().min(1).optional(),
        docType: z.enum(["guide", "reference", "architecture", "runbook"]).optional(),
      }),
    }),
  }),
};
