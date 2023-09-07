-- CreateTable
CREATE TABLE "user" (
    "id" TEXT NOT NULL,
    "username" VARCHAR(140) NOT NULL,
    "email" VARCHAR(140) NOT NULL,
    "first_name" VARCHAR(140) NOT NULL,
    "last_name" VARCHAR(140) NOT NULL,
    "is_active" BOOLEAN NOT NULL DEFAULT true,
    "created" TIMESTAMPTZ(6) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated" TIMESTAMPTZ(6),

    CONSTRAINT "user_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "role" (
    "id" TEXT NOT NULL,
    "name" VARCHAR(140) NOT NULL,
    "description" VARCHAR(255) NOT NULL,

    CONSTRAINT "role_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "guest_role" (
    "id" TEXT NOT NULL,
    "name" VARCHAR(140) NOT NULL,
    "description" VARCHAR(255),
    "role_id" TEXT NOT NULL,
    "permissions" INTEGER[],

    CONSTRAINT "guest_role_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "guest_user" (
    "id" TEXT NOT NULL,
    "email" TEXT NOT NULL,
    "guest_role_id" TEXT NOT NULL,
    "created" TIMESTAMPTZ(6) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated" TIMESTAMPTZ(6),

    CONSTRAINT "guest_user_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "account_type" (
    "id" TEXT NOT NULL,
    "name" VARCHAR(128) NOT NULL,
    "description" VARCHAR(255) NOT NULL,
    "is_subscription" BOOLEAN NOT NULL DEFAULT false,
    "is_manager" BOOLEAN NOT NULL DEFAULT false,
    "is_staff" BOOLEAN NOT NULL DEFAULT false,

    CONSTRAINT "account_type_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "account" (
    "id" TEXT NOT NULL,
    "name" VARCHAR(64) NOT NULL,
    "owner_id" TEXT NOT NULL,
    "account_type_id" TEXT NOT NULL,
    "created" TIMESTAMPTZ(6) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated" TIMESTAMPTZ(6),
    "is_active" BOOLEAN NOT NULL DEFAULT true,
    "is_checked" BOOLEAN NOT NULL DEFAULT false,
    "is_archived" BOOLEAN NOT NULL DEFAULT false,

    CONSTRAINT "account_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "guest_user_on_account" (
    "guest_user_id" TEXT NOT NULL,
    "account_id" TEXT NOT NULL,
    "created" TIMESTAMPTZ(6) NOT NULL DEFAULT CURRENT_TIMESTAMP,

    CONSTRAINT "guest_user_on_account_pkey" PRIMARY KEY ("guest_user_id","account_id")
);

-- CreateTable
CREATE TABLE "error_code" (
    "code" SERIAL NOT NULL,
    "prefix" TEXT NOT NULL,
    "message" VARCHAR(255) NOT NULL,
    "details" VARCHAR(255),
    "is_internal" BOOLEAN NOT NULL DEFAULT false,
    "is_native" BOOLEAN NOT NULL DEFAULT false,

    CONSTRAINT "error_code_pkey" PRIMARY KEY ("prefix","code")
);

-- CreateTable
CREATE TABLE "webhook" (
    "id" TEXT NOT NULL,
    "name" VARCHAR(140) NOT NULL,
    "description" VARCHAR(255) NOT NULL,
    "target" VARCHAR(15) NOT NULL,
    "url" VARCHAR(255) NOT NULL,
    "is_active" BOOLEAN NOT NULL DEFAULT true,
    "created" TIMESTAMPTZ(6) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated" TIMESTAMPTZ(6),

    CONSTRAINT "webhook_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "webhook_action" (
    "id" TEXT NOT NULL,
    "name" VARCHAR(64) NOT NULL,
    "description" VARCHAR(255) NOT NULL,
    "created" TIMESTAMPTZ(6) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated" TIMESTAMPTZ(6),
    "webhook_id" TEXT NOT NULL,

    CONSTRAINT "webhook_action_pkey" PRIMARY KEY ("id")
);

-- CreateIndex
CREATE UNIQUE INDEX "user_email_key" ON "user"("email");

-- CreateIndex
CREATE UNIQUE INDEX "guest_role_name_key" ON "guest_role"("name");

-- CreateIndex
CREATE UNIQUE INDEX "guest_user_email_guest_role_id_key" ON "guest_user"("email", "guest_role_id");

-- CreateIndex
CREATE UNIQUE INDEX "account_type_name_key" ON "account_type"("name");

-- CreateIndex
CREATE UNIQUE INDEX "account_owner_id_key" ON "account"("owner_id");

-- CreateIndex
CREATE UNIQUE INDEX "account_name_owner_id_key" ON "account"("name", "owner_id");

-- CreateIndex
CREATE UNIQUE INDEX "webhook_url_target_key" ON "webhook"("url", "target");

-- CreateIndex
CREATE UNIQUE INDEX "webhook_action_name_webhook_id_key" ON "webhook_action"("name", "webhook_id");

-- AddForeignKey
ALTER TABLE "guest_role" ADD CONSTRAINT "guest_role_role_id_fkey" FOREIGN KEY ("role_id") REFERENCES "role"("id") ON DELETE RESTRICT ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "guest_user" ADD CONSTRAINT "guest_user_guest_role_id_fkey" FOREIGN KEY ("guest_role_id") REFERENCES "guest_role"("id") ON DELETE RESTRICT ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "account" ADD CONSTRAINT "account_owner_id_fkey" FOREIGN KEY ("owner_id") REFERENCES "user"("id") ON DELETE RESTRICT ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "account" ADD CONSTRAINT "account_account_type_id_fkey" FOREIGN KEY ("account_type_id") REFERENCES "account_type"("id") ON DELETE RESTRICT ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "guest_user_on_account" ADD CONSTRAINT "guest_user_on_account_guest_user_id_fkey" FOREIGN KEY ("guest_user_id") REFERENCES "guest_user"("id") ON DELETE CASCADE ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "guest_user_on_account" ADD CONSTRAINT "guest_user_on_account_account_id_fkey" FOREIGN KEY ("account_id") REFERENCES "account"("id") ON DELETE RESTRICT ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "webhook_action" ADD CONSTRAINT "webhook_action_webhook_id_fkey" FOREIGN KEY ("webhook_id") REFERENCES "webhook"("id") ON DELETE RESTRICT ON UPDATE CASCADE;
