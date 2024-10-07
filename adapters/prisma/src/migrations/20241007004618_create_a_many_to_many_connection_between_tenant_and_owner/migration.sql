/*
  Warnings:

  - You are about to drop the column `tenant_id` on the `user` table. All the data in the column will be lost.

*/
-- DropForeignKey
ALTER TABLE "user" DROP CONSTRAINT "user_tenant_id_fkey";

-- AlterTable
ALTER TABLE "user" DROP COLUMN "tenant_id";

-- CreateTable
CREATE TABLE "owner_on_tenant" (
    "id" TEXT NOT NULL,
    "tenant_id" TEXT NOT NULL,
    "owner_id" TEXT NOT NULL,
    "guest_by" TEXT NOT NULL,
    "created" TIMESTAMPTZ(6) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated" TIMESTAMPTZ(6),

    CONSTRAINT "owner_on_tenant_pkey" PRIMARY KEY ("id")
);

-- CreateIndex
CREATE UNIQUE INDEX "owner_on_tenant_tenant_id_key" ON "owner_on_tenant"("tenant_id");

-- CreateIndex
CREATE UNIQUE INDEX "owner_on_tenant_owner_id_key" ON "owner_on_tenant"("owner_id");

-- CreateIndex
CREATE UNIQUE INDEX "owner_on_tenant_tenant_id_owner_id_key" ON "owner_on_tenant"("tenant_id", "owner_id");

-- AddForeignKey
ALTER TABLE "owner_on_tenant" ADD CONSTRAINT "owner_on_tenant_tenant_id_fkey" FOREIGN KEY ("tenant_id") REFERENCES "tenant"("id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "owner_on_tenant" ADD CONSTRAINT "owner_on_tenant_owner_id_fkey" FOREIGN KEY ("owner_id") REFERENCES "user"("id") ON DELETE CASCADE ON UPDATE CASCADE;
