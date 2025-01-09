/*
  Warnings:

  - A unique constraint covering the columns `[slug,tenant_id]` on the table `account` will be added. If there are existing duplicate values, this will fail.

*/
-- CreateIndex
CREATE UNIQUE INDEX "account_slug_tenant_id_key" ON "account"("slug", "tenant_id");
