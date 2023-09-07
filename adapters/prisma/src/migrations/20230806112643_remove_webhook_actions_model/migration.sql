/*
  Warnings:

  - You are about to drop the `webhook_action` table. If the table is not empty, all the data it contains will be lost.

*/
-- DropForeignKey
ALTER TABLE "webhook_action" DROP CONSTRAINT "webhook_action_webhook_id_fkey";

-- DropTable
DROP TABLE "webhook_action";
