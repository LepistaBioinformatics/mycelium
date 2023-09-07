-- DropForeignKey
ALTER TABLE "webhook_action" DROP CONSTRAINT "webhook_action_webhook_id_fkey";

-- AddForeignKey
ALTER TABLE "webhook_action" ADD CONSTRAINT "webhook_action_webhook_id_fkey" FOREIGN KEY ("webhook_id") REFERENCES "webhook"("id") ON DELETE CASCADE ON UPDATE CASCADE;
