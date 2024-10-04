/*
  Warnings:

  - You are about to drop the column `account_type_id` on the `account` table. All the data in the column will be lost.
  - You are about to drop the `account_type` table. If the table is not empty, all the data it contains will be lost.

*/
-- DropForeignKey
ALTER TABLE "account" DROP CONSTRAINT "account_account_type_id_fkey";

-- AlterTable
ALTER TABLE "account" DROP COLUMN "account_type_id",
ADD COLUMN     "account_type" JSON NOT NULL DEFAULT '{}';

-- AlterTable
ALTER TABLE "user" ADD COLUMN     "tenant_id" TEXT;

-- DropTable
DROP TABLE "account_type";

-- CreateTable
CREATE TABLE "Tenant" (
    "id" TEXT NOT NULL,
    "name" VARCHAR(255) NOT NULL,
    "description" TEXT,
    "meta" JSONB,
    "status" JSONB[],
    "manager_id" TEXT,

    CONSTRAINT "Tenant_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "tenant_tag" (
    "id" TEXT NOT NULL,
    "value" VARCHAR(64) NOT NULL,
    "meta" JSONB,
    "tenant_id" TEXT NOT NULL,

    CONSTRAINT "tenant_tag_pkey" PRIMARY KEY ("id")
);

-- CreateIndex
CREATE UNIQUE INDEX "tenant_tag_value_tenant_id_key" ON "tenant_tag"("value", "tenant_id");

-- AddForeignKey
ALTER TABLE "Tenant" ADD CONSTRAINT "Tenant_manager_id_fkey" FOREIGN KEY ("manager_id") REFERENCES "account"("id") ON DELETE SET NULL ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "tenant_tag" ADD CONSTRAINT "tenant_tag_tenant_id_fkey" FOREIGN KEY ("tenant_id") REFERENCES "Tenant"("id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "user" ADD CONSTRAINT "user_tenant_id_fkey" FOREIGN KEY ("tenant_id") REFERENCES "Tenant"("id") ON DELETE SET NULL ON UPDATE CASCADE;
