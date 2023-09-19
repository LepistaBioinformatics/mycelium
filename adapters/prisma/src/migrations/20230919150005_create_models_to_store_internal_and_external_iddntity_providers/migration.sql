-- CreateTable
CREATE TABLE "internal_provider" (
    "password_hash" VARCHAR(255) NOT NULL,
    "password_salt" VARCHAR(255) NOT NULL,
    "user_id" TEXT NOT NULL,

    CONSTRAINT "internal_provider_pkey" PRIMARY KEY ("user_id")
);

-- CreateTable
CREATE TABLE "external_provider" (
    "name" VARCHAR(255) NOT NULL,
    "user_id" TEXT NOT NULL,

    CONSTRAINT "external_provider_pkey" PRIMARY KEY ("user_id")
);

-- CreateIndex
CREATE UNIQUE INDEX "internal_provider_user_id_password_hash_password_salt_key" ON "internal_provider"("user_id", "password_hash", "password_salt");

-- CreateIndex
CREATE UNIQUE INDEX "external_provider_user_id_name_key" ON "external_provider"("user_id", "name");

-- AddForeignKey
ALTER TABLE "internal_provider" ADD CONSTRAINT "internal_provider_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "user"("id") ON DELETE RESTRICT ON UPDATE CASCADE;

-- AddForeignKey
ALTER TABLE "external_provider" ADD CONSTRAINT "external_provider_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "user"("id") ON DELETE RESTRICT ON UPDATE CASCADE;
