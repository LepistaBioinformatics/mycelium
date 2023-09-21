-- DropForeignKey
ALTER TABLE "user" DROP CONSTRAINT "user_account_id_fkey";

-- AlterTable
ALTER TABLE "user" ALTER COLUMN "account_id" DROP NOT NULL;

-- AddForeignKey
ALTER TABLE "user" ADD CONSTRAINT "user_account_id_fkey" FOREIGN KEY ("account_id") REFERENCES "account"("id") ON DELETE SET NULL ON UPDATE CASCADE;
