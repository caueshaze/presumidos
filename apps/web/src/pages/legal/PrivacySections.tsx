import { Card } from "@/components/ui/card";
import { PrivacyDataSections } from "./PrivacyDataSections";
import { PrivacyRightsSections } from "./PrivacyRightsSections";

export function PrivacySections() {
  return (
    <Card className="space-y-6">
      <PrivacyDataSections />
      <PrivacyRightsSections />
    </Card>
  );
}
