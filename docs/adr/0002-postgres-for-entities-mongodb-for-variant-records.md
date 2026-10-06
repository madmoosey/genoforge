# ADR 0002: Postgres for entities with lifecycle, MongoDB for variant records

**Status:** accepted · **Date:** 2026-10-06

## Context

The pipeline produces two very different kinds of data:

1. **Entities with identity, lifecycle and joins** — samples, sequencing runs,
   pipeline runs, their steps, produced artefacts and summary QC metrics. These
   are queried by relationship ("all failed steps for runs of this sample"),
   need constraints (a step attempt is unique per run), and are edited by
   people through an admin.
2. **High-cardinality, semi-structured records** — one document per variant
   call. VCF INFO and FORMAT fields vary by caller and version, and downstream
   annotation tools add arbitrary nested fields later. Access is by
   `(run, chrom, pos)` range and by a handful of indexed scalars.

## Decision

- Postgres, through the Django ORM with committed migrations, holds category 1.
  Constraints live on the models. A JSONB `details` column on `QcMetric`
  absorbs the long tail of per-stage metrics without schema churn.
- MongoDB, through `pymongo` (not an ORM shim), holds category 2 in a
  `variants` collection with a deterministic `_id` of
  `run:<uuid>:<chrom>:<pos>:<ref>:<alt>` so loads are idempotent. Indexes are
  created on application start-up and documented in `SCHEMA.md`.

## Alternatives considered

- **Postgres only, variants in JSONB** — workable at phiX scale, but a
  human-genome VCF is millions of rows per sample; a document store with
  compound indexes and aggregation pipelines is the better fit and shows the
  skill explicitly.
- **MongoDB only** — loses foreign-key integrity and the admin for the
  entities that need them most.

## Consequences

- Two services in `docker-compose.yml`; readiness checks must ping both.
- No cross-database transactions. The loader writes Mongo first and marks the
  Postgres step succeeded last; a retry re-upserts by `_id` safely.
