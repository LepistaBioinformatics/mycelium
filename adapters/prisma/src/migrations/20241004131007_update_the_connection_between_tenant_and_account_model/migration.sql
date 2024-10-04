/*
  Warnings:

  - You are about to drop the column `manager_id` on the `tenant` table. All the data in the column will be lost.
  - A unique constraint covering the columns `[name,tenant_id]` on the table `account` will be added. If there are existing duplicate values, this will fail.
  - A unique constraint covering the columns `[name]` on the table `tenant` will be added. If there are existing duplicate values, this will fail.
  - Added the required column `tenant_id` to the `account` table without a default value. This is not possible if the table is not empty.

*/
-- DropForeignKey
ALTER TABLE "tenant" DROP CONSTRAINT "tenant_manager_id_fkey";

-- DropIndex
DROP INDEX "account_name_key";

-- DropIndex
DROP INDEX "tenant_name_manager_id_key";

-- AlterTable
ALTER TABLE "account" ADD COLUMN     "tenant_id" TEXT NOT NULL;

-- AlterTable
ALTER TABLE "tenant" DROP COLUMN "manager_id",
ADD COLUMN     "created" TIMESTAMPTZ(6) NOT NULL DEFAULT CURRENT_TIMESTAMP,
ADD COLUMN     "updated" TIMESTAMPTZ(6);

-- CreateTable
CREATE TABLE "manager_account_on_tenant" (
    "id" TEXT NOT NULL,
    "tenant_id" TEXT NOT NULL,
    "account_id" TEXT NOT NULL,
    "created" TIMESTAMPTZ(6) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated" TIMESTAMPTZ(6),

    CONSTRAINT "manager_account_on_tenant_pkey" PRIMARY KEY ("id")
);

-- CreateIndex
CREATE UNIQUE INDEX "manager_account_on_tenant_tenant_id_key" ON "manager_account_on_tenant"("tenant_id");

-- CreateIndex
CREATE UNIQUE INDEX "manager_account_on_tenant_account_id_key" ON "manager_account_on_tenant"("account_id");

-- CreateIndex
CREATE UNIQUE INDEX "manager_account_on_tenant_tenant_id_account_id_key" ON "manager_account_on_tenant"("tenant_id", "account_id");

-- CreateIndex
CREATE UNIQUE INDEX "account_name_tenant_id_key" ON "account"("name", "tenant_id");

-- CreateIndex
CREATE UNIQUE INDEX "tenant_name_key" ON "tenant"("name");

-- AddForeignKey
ALTER TABLE "manager_account_on_tenant" ADD CONSTRAINT "manager_account_on_tenant_tenant_id_fkey" FOREIGN KEY ("tenant_id") REFERENCES "tenant"("id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "manager_account_on_tenant" ADD CONSTRAINT "manager_account_on_tenant_account_id_fkey" FOREIGN KEY ("account_id") REFERENCES "account"("id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "account" ADD CONSTRAINT "account_tenant_id_fkey" FOREIGN KEY ("tenant_id") REFERENCES "tenant"("id") ON DELETE RESTRICT ON UPDATE CASCADE;
