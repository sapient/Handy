import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { useSettings } from "../../hooks/useSettings";
import { Input } from "../ui/Input";
import { Button } from "../ui/Button";
import { SettingContainer } from "../ui/SettingContainer";

export const TriggerWords: React.FC = React.memo(() => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();
  const [newWord, setNewWord] = useState("");
  const triggerWords = getSetting("post_process_trigger_words") || [];

  const handleAddWord = () => {
    const trimmed = newWord.trim();
    if (!trimmed || trimmed.includes(" ") || trimmed.length > 50) return;
    if (triggerWords.some((w) => w.toLowerCase() === trimmed.toLowerCase())) {
      toast.error(
        t("settings.postProcessing.triggerWords.duplicate", { word: trimmed }),
      );
      return;
    }
    updateSetting("post_process_trigger_words", [...triggerWords, trimmed]);
    setNewWord("");
  };

  const handleRemoveWord = (word: string) => {
    updateSetting(
      "post_process_trigger_words",
      triggerWords.filter((w) => w !== word),
    );
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") {
      e.preventDefault();
      handleAddWord();
    }
  };

  return (
    <>
      <SettingContainer
        title={t("settings.postProcessing.triggerWords.title")}
        description={t("settings.postProcessing.triggerWords.description")}
        descriptionMode="tooltip"
        grouped={true}
      >
        <div className="flex items-center gap-2">
          <Input
            type="text"
            className="max-w-40"
            value={newWord}
            onChange={(e) => setNewWord(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder={t(
              "settings.postProcessing.triggerWords.placeholder",
            )}
            variant="compact"
            disabled={isUpdating("post_process_trigger_words")}
          />
          <Button
            onClick={handleAddWord}
            disabled={
              !newWord.trim() ||
              newWord.includes(" ") ||
              newWord.trim().length > 50 ||
              isUpdating("post_process_trigger_words")
            }
            variant="primary"
            size="md"
          >
            {t("settings.postProcessing.triggerWords.add")}
          </Button>
        </div>
      </SettingContainer>
      {triggerWords.length > 0 && (
        <div className="px-4 p-2 flex flex-wrap gap-1">
          {triggerWords.map((word) => (
            <Button
              key={word}
              onClick={() => handleRemoveWord(word)}
              disabled={isUpdating("post_process_trigger_words")}
              variant="secondary"
              size="sm"
              className="inline-flex items-center gap-1 cursor-pointer"
              aria-label={t("settings.postProcessing.triggerWords.remove", {
                word,
              })}
            >
              <span>{word}</span>
              <svg
                className="w-3 h-3"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M6 18L18 6M6 6l12 12"
                />
              </svg>
            </Button>
          ))}
        </div>
      )}
    </>
  );
});

TriggerWords.displayName = "TriggerWords";
