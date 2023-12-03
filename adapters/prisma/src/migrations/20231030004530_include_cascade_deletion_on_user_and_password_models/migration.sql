-- DropForeignKey
ALTER TABLE "identity_provider" DROP CONSTRAINT "identity_provider_user_id_fkey";

-- AddForeignKey
ALTER TABLE "identity_provider" ADD CONSTRAINT "identity_provider_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "user"("id") ON DELETE CASCADE ON UPDATE CASCADE;
