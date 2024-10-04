-- DropForeignKey
ALTER TABLE "account" DROP CONSTRAINT "account_tenant_id_fkey";

-- AlterTable
ALTER TABLE "account" ALTER COLUMN "tenant_id" DROP NOT NULL;

-- AddForeignKey
ALTER TABLE "account" ADD CONSTRAINT "account_tenant_id_fkey" FOREIGN KEY ("tenant_id") REFERENCES "tenant"("id") ON DELETE SET NULL ON UPDATE CASCADE;
